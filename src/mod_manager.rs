use crate::content::ModManifest;
use crate::game_paths::GamePaths;
use atomic_write_file::AtomicWriteFile;
use flate2::read::DeflateDecoder;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: usize = 256 * 1024 * 1024;
const MAX_ENTRY_BYTES: usize = 64 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4096;

#[derive(Debug, Clone)]
pub(crate) struct InstalledMod {
    pub manifest: ModManifest,
    pub directory: PathBuf,
    pub enabled: bool,
    pub priority: i32,
    pub asset_count: usize,
}

static ACTIVE_MODS: std::sync::OnceLock<Vec<InstalledMod>> = std::sync::OnceLock::new();

/// Capture the mod set used by the current process. Settings edits made from
/// Options are intentionally not reflected until the next full initialization.
pub(crate) fn initialize_active_mods() {
    if ACTIVE_MODS.get().is_some() {
        return;
    }
    if let Err(error) = apply_pending_removals() {
        eprintln!("warning: could not finish pending mod removals: {error}");
    }
    let mods = match list_installed_mods() {
        Ok(mods) => mods,
        Err(error) => {
            eprintln!("warning: could not initialize the active mod set: {error}");
            Vec::new()
        }
    };
    let _ = ACTIVE_MODS.set(mods);
}

pub(crate) fn active_mods() -> &'static [InstalledMod] {
    ACTIVE_MODS.get().map(Vec::as_slice).unwrap_or(&[])
}

fn apply_pending_removals() -> io::Result<()> {
    let paths = GamePaths::get();
    let mut settings = read_settings(&paths.mod_settings_path)?;
    if settings.removed_mods.is_empty() {
        return Ok(());
    }
    let mods_root = fs::canonicalize(&paths.mods_dir)?;
    let mut completed = Vec::new();

    for id in &settings.removed_mods {
        let candidate = paths.mods_dir.join(id);
        if !candidate.exists() {
            completed.push(id.clone());
            continue;
        }
        let canonical = fs::canonicalize(&candidate)?;
        if !canonical.starts_with(&mods_root) || canonical == mods_root {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("refusing to remove mod outside managed storage: {id}"),
            ));
        }
        fs::remove_dir_all(canonical)?;
        completed.push(id.clone());
    }

    for id in completed {
        settings.removed_mods.remove(&id);
        settings.mods.remove(&id);
    }
    write_settings(&paths.mod_settings_path, &settings)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ModSettingsFile {
    #[serde(default)]
    mods: BTreeMap<String, ModPreferences>,
    #[serde(default)]
    removed_mods: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModPreferences {
    enabled: bool,
    priority: i32,
}

#[derive(Debug)]
struct ArchiveEntry {
    path: String,
    is_directory: bool,
    contents: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct AssetOverrideConflict {
    pub path: String,
    pub providers: Vec<(String, i32)>,
    pub winner: String,
}

pub(crate) fn list_installed_mods() -> io::Result<Vec<InstalledMod>> {
    let paths = GamePaths::get();
    let settings = read_settings(&paths.mod_settings_path)?;
    let mut installed = Vec::new();

    for entry in fs::read_dir(&paths.mods_dir)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                bevy::log::warn!("Could not inspect a mod directory: {error}");
                continue;
            }
        };
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }

        let manifest_path = directory.join("manifest.json");
        let manifest_data = match fs::read(&manifest_path) {
            Ok(data) => data,
            Err(error) => {
                bevy::log::warn!(
                    "Skipping mod without a readable manifest at {}: {error}",
                    manifest_path.display()
                );
                continue;
            }
        };
        let mut manifest: ModManifest = match serde_json::from_slice(&manifest_data) {
            Ok(manifest) => manifest,
            Err(error) => {
                bevy::log::warn!(
                    "Skipping mod with invalid manifest {}: {error}",
                    manifest_path.display()
                );
                continue;
            }
        };
        if let Err(error) = validate_manifest(&manifest) {
            bevy::log::warn!("Skipping invalid mod at {}: {error}", directory.display());
            continue;
        }

        if settings.removed_mods.contains(&manifest.id) {
            continue;
        }
        let preferences = settings.mods.get(&manifest.id);
        let enabled = preferences
            .map(|value| value.enabled)
            .unwrap_or(manifest.enabled);
        let priority = preferences
            .map(|value| value.priority)
            .unwrap_or(manifest.priority);
        manifest.enabled = enabled;
        manifest.priority = priority;

        let asset_count = count_asset_files(&directory.join("assets")).unwrap_or(0);
        installed.push(InstalledMod {
            manifest,
            directory,
            enabled,
            priority,
            asset_count,
        });
    }

    installed.sort_by(|left, right| {
        left.manifest
            .id
            .cmp(&right.manifest.id)
            .then_with(|| left.directory.cmp(&right.directory))
    });
    let mut seen_ids = HashSet::new();
    installed.retain(|item| {
        if seen_ids.insert(item.manifest.id.clone()) {
            true
        } else {
            bevy::log::warn!(
                "Skipping duplicate installed mod ID {} at {}",
                item.manifest.id,
                item.directory.display()
            );
            false
        }
    });
    installed.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.manifest.id.cmp(&right.manifest.id))
    });
    Ok(installed)
}

pub(crate) fn set_mod_enabled(id: &str, enabled: bool) -> io::Result<()> {
    update_preferences(id, |preferences| preferences.enabled = enabled)
}

pub(crate) fn set_mod_priority(id: &str, priority: i32) -> io::Result<()> {
    update_preferences(id, |preferences| preferences.priority = priority)
}

pub(crate) fn remove_mod(id: &str) -> io::Result<()> {
    let installed = list_installed_mods()?
        .into_iter()
        .find(|item| item.manifest.id == id)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("mod {id} is not installed"),
            )
        })?;

    let mods_root = fs::canonicalize(&GamePaths::get().mods_dir)?;
    let mod_root = fs::canonicalize(&installed.directory)?;
    if !mod_root.starts_with(&mods_root) || mod_root == mods_root {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "refusing to remove a mod outside the managed mods directory",
        ));
    }

    let mut settings = read_settings(&GamePaths::get().mod_settings_path)?;
    settings.mods.remove(id);
    settings.removed_mods.insert(id.to_string());
    // Keep the package on disk until process restart so the active runtime
    // snapshot can continue resolving its assets for the current campaign.
    write_settings(&GamePaths::get().mod_settings_path, &settings)
}

pub(crate) fn install_mod_archive(archive_path: &Path) -> io::Result<InstalledMod> {
    let metadata = fs::metadata(archive_path)?;
    if metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mod archive exceeds the 128 MiB compressed-size limit",
        ));
    }
    let mut archive = Vec::with_capacity(metadata.len() as usize);
    File::open(archive_path)?
        .take(MAX_ARCHIVE_BYTES + 1)
        .read_to_end(&mut archive)?;
    if archive.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mod archive is too large",
        ));
    }

    let entries = parse_zip_archive(&archive)?;
    let prefix = archive_root_prefix(&entries)?;
    let paths = GamePaths::get();
    let staging_root = paths.root.join(".mod-staging");
    fs::create_dir_all(&staging_root)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let staging = staging_root.join(format!("install-{stamp}"));
    fs::create_dir(&staging)?;

    let extraction_result = extract_entries(entries, prefix.as_deref(), &staging)
        .and_then(|_| validate_staged_mod(&staging));
    let manifest = match extraction_result {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };

    let target = paths.mods_dir.join(&manifest.id);
    let installed = match list_installed_mods() {
        Ok(installed) => installed,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    if installed.iter().any(|item| item.manifest.id == manifest.id) || target.exists() {
        let _ = fs::remove_dir_all(&staging);
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "mod {} is already installed; remove it before installing a replacement",
                manifest.id
            ),
        ));
    }

    if let Err(error) = fs::rename(&staging, &target) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let settings_result = update_preferences(&manifest.id, |preferences| {
        preferences.enabled = manifest.enabled;
        preferences.priority = manifest.priority;
    });
    if let Err(error) = settings_result {
        let _ = fs::remove_dir_all(&target);
        return Err(error);
    }

    list_installed_mods()?
        .into_iter()
        .find(|item| item.manifest.id == manifest.id)
        .ok_or_else(|| io::Error::other("installed mod could not be rediscovered"))
}

pub(crate) fn resolve_asset_reference(reference: &str) -> io::Result<PathBuf> {
    let installed = active_mods();

    if let Some(mod_reference) = reference.strip_prefix("mod:") {
        let (id, relative) = mod_reference.split_once('/').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "mod asset reference must include a mod ID and file path",
            )
        })?;
        let relative = validate_relative_path(relative)?;
        let item = installed
            .iter()
            .find(|item| item.enabled && item.manifest.id == id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("asset provider mod {id} is not enabled"),
                )
            })?;
        return resolve_inside(&item.directory.join("assets"), &relative);
    }

    let relative = validate_relative_path(reference)?;
    let mut providers = installed
        .iter()
        .filter(|item| item.enabled)
        .collect::<Vec<_>>();
    providers.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| right.manifest.id.cmp(&left.manifest.id))
    });

    for provider in providers {
        let candidate = provider.directory.join("assets").join(&relative);
        if candidate.is_file() {
            return resolve_inside(&provider.directory.join("assets"), &relative);
        }
    }

    resolve_inside(&GamePaths::get().bundled_assets_dir, &relative)
}

pub(crate) fn asset_override_conflicts() -> io::Result<Vec<AssetOverrideConflict>> {
    let installed = active_mods();
    let mut by_path: BTreeMap<String, Vec<(String, i32)>> = BTreeMap::new();

    for item in installed.iter().filter(|item| item.enabled) {
        let assets = item.directory.join("assets");
        let mut files = Vec::new();
        collect_files(&assets, &assets, &mut files)?;
        for relative in files {
            by_path
                .entry(relative.to_string_lossy().replace('\\', "/"))
                .or_default()
                .push((item.manifest.id.clone(), item.priority));
        }
    }

    let mut conflicts = Vec::new();
    for (path, mut providers) in by_path {
        if providers.len() < 2 {
            continue;
        }
        providers.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
        let winner = providers
            .last()
            .map(|entry| entry.0.clone())
            .unwrap_or_default();
        conflicts.push(AssetOverrideConflict {
            path,
            providers,
            winner,
        });
    }
    Ok(conflicts)
}

fn read_settings(path: &Path) -> io::Result<ModSettingsFile> {
    if !path.is_file() {
        return Ok(ModSettingsFile::default());
    }
    let data = fs::read(path)?;
    serde_json::from_slice(&data).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("mod settings file is invalid: {error}"),
        )
    })
}

fn write_settings(path: &Path, settings: &ModSettingsFile) -> io::Result<()> {
    let data =
        serde_json::to_vec_pretty(settings).map_err(|error| io::Error::other(error.to_string()))?;
    let mut file = AtomicWriteFile::open(path)?;
    file.write_all(&data)?;
    file.commit()?;
    Ok(())
}

fn update_preferences(id: &str, update: impl FnOnce(&mut ModPreferences)) -> io::Result<()> {
    let installed = list_installed_mods()?
        .into_iter()
        .find(|item| item.manifest.id == id)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("mod {id} is not installed"),
            )
        })?;
    let path = &GamePaths::get().mod_settings_path;
    let mut settings = read_settings(path)?;
    let preferences = settings
        .mods
        .entry(id.to_string())
        .or_insert(ModPreferences {
            enabled: installed.enabled,
            priority: installed.priority,
        });
    update(preferences);
    write_settings(path, &settings)
}

pub(crate) fn validate_manifest(manifest: &ModManifest) -> io::Result<()> {
    let id = manifest.id.as_str();
    if id.is_empty()
        || id.len() > 128
        || id.starts_with('.')
        || id.contains("..")
        || !id.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-' | '_')
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mod ID must use lowercase ASCII letters, digits, dots, underscores, or hyphens",
        ));
    }
    if manifest.name.trim().is_empty() || manifest.name.len() > 160 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mod name must contain 1 to 160 bytes",
        ));
    }
    if manifest.version.trim().is_empty() || manifest.version.len() > 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mod version must contain 1 to 64 bytes",
        ));
    }
    if let Some(content_file) = manifest.content_file.as_deref() {
        validate_relative_path(content_file)?;
    }
    Ok(())
}

fn validate_staged_mod(staging: &Path) -> io::Result<ModManifest> {
    let manifest_path = staging.join("manifest.json");
    let data = fs::read(&manifest_path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("mod archive is missing manifest.json: {error}"),
        )
    })?;
    let manifest: ModManifest = serde_json::from_slice(&data).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid mod manifest: {error}"),
        )
    })?;
    validate_manifest(&manifest)?;

    let mut has_payload = false;
    if let Some(content_file) = manifest.content_file.as_deref() {
        let relative = validate_relative_path(content_file)?;
        let root = fs::canonicalize(staging)?;
        let content_path = resolve_inside(&root, &relative)?;
        let content_data = fs::read(content_path)?;
        let content: crate::content::CampaignContent = serde_json::from_slice(&content_data)
            .map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid mod content: {error}"),
                )
            })?;
        if content.version != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported mod content version {}", content.version),
            ));
        }
        // Cross-mod references are validated after all active content has been
        // merged. A mod is allowed to reference base or another mod's content.
        has_payload = true;
    }

    let assets = staging.join("assets");
    let mut asset_files = Vec::new();
    if assets.is_dir() {
        collect_files(&assets, &assets, &mut asset_files)?;
        for relative in &asset_files {
            if relative
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("png")
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unsupported mod asset format: {}", relative.display()),
                ));
            }
            validate_png_header(&assets.join(relative))?;
        }
        has_payload |= !asset_files.is_empty();
    }

    if !has_payload {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mod must include content_file or at least one PNG in assets/",
        ));
    }

    Ok(manifest)
}

fn validate_png_header(path: &Path) -> io::Result<()> {
    let mut file = File::open(path)?;
    let mut header = [0u8; 24];
    file.read_exact(&mut header).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid PNG {}: {error}", path.display()),
        )
    })?;
    if &header[..8] != b"\x89PNG\r\n\x1a\n" || &header[12..16] != b"IHDR" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid PNG header: {}", path.display()),
        ));
    }
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "PNG dimensions are invalid or too large: {}",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> io::Result<PathBuf> {
    if value.is_empty()
        || value.len() > 1024
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains(':')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path must be a safe relative path of at most 1024 bytes",
        ));
    }
    let path = Path::new(value);
    let components = path.components().collect::<Vec<_>>();
    if components.is_empty()
        || components.len() > 64
        || !components
            .iter()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path traversal or excessive path depth is not allowed",
        ));
    }
    Ok(path.to_path_buf())
}

fn resolve_inside(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical_path = fs::canonicalize(canonical_root.join(relative))?;
    if !canonical_path.starts_with(&canonical_root) || !canonical_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "resolved asset or content path escapes its managed directory",
        ));
    }
    Ok(canonical_path)
}

fn count_asset_files(root: &Path) -> io::Result<usize> {
    let mut files = Vec::new();
    if root.is_dir() {
        collect_files(root, root, &mut files)?;
    }
    Ok(files.len())
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if !current.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

fn archive_root_prefix(entries: &[ArchiveEntry]) -> io::Result<Option<String>> {
    if entries
        .iter()
        .any(|entry| !entry.is_directory && entry.path == "manifest.json")
    {
        return Ok(None);
    }
    let mut root: Option<String> = None;
    for entry in entries {
        let first = entry.path.split('/').next().unwrap_or_default();
        if first.is_empty() {
            continue;
        }
        match root.as_deref() {
            Some(existing) if existing != first => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mod ZIP must contain one top-level directory or a root manifest.json",
                ));
            }
            None => root = Some(first.to_string()),
            _ => {}
        }
    }
    root.map(Some)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "mod ZIP is empty"))
}

fn extract_entries(
    entries: Vec<ArchiveEntry>,
    prefix: Option<&str>,
    destination: &Path,
) -> io::Result<()> {
    let mut seen = HashSet::new();
    let mut total_size = 0usize;
    for entry in entries {
        let relative_name = match prefix {
            Some(prefix) => {
                let prefix_with_slash = format!("{prefix}/");
                if entry.path == prefix || entry.path == format!("{prefix}/") {
                    continue;
                }
                entry.path.strip_prefix(&prefix_with_slash).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "ZIP entry is outside the package root",
                    )
                })?
            }
            None => entry.path.as_str(),
        };
        if relative_name.is_empty() {
            continue;
        }
        let trimmed = relative_name.trim_end_matches('/');
        let relative = validate_relative_path(trimmed)?;
        let key = relative.to_string_lossy().to_string();
        if !seen.insert(key) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ZIP contains duplicate paths",
            ));
        }

        let target = destination.join(&relative);
        if entry.is_directory {
            fs::create_dir_all(&target)?;
            continue;
        }
        total_size = total_size.saturating_add(entry.contents.len());
        if total_size > MAX_EXTRACTED_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "mod expands beyond the 256 MiB limit",
            ));
        }
        let parent = target.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "ZIP file has no parent directory",
            )
        })?;
        fs::create_dir_all(parent)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        file.write_all(&entry.contents)?;
    }
    Ok(())
}

fn parse_zip_archive(data: &[u8]) -> io::Result<Vec<ArchiveEntry>> {
    let search_start = data.len().saturating_sub(65_557);
    let eocd = (search_start..data.len().saturating_sub(3))
        .rev()
        .find(|offset| read_u32(data, *offset).ok() == Some(0x0605_4b50))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "ZIP end-of-central-directory record not found",
            )
        })?;

    let disk_number = read_u16(data, eocd + 4)?;
    let central_disk = read_u16(data, eocd + 6)?;
    let disk_entries = read_u16(data, eocd + 8)?;
    let total_entries = read_u16(data, eocd + 10)?;
    let central_size = read_u32(data, eocd + 12)? as usize;
    let central_offset = read_u32(data, eocd + 16)? as usize;
    let comment_length = read_u16(data, eocd + 20)? as usize;
    if eocd
        .checked_add(22)
        .and_then(|offset| offset.checked_add(comment_length))
        != Some(data.len())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ZIP end record comment length is invalid",
        ));
    }
    if disk_number != 0 || central_disk != 0 || disk_entries != total_entries {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "multi-disk ZIP archives are not supported",
        ));
    }
    if total_entries == u16::MAX
        || central_size == u32::MAX as usize
        || central_offset == u32::MAX as usize
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ZIP64 archives are not supported",
        ));
    }
    let central_end = central_offset.checked_add(central_size).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "ZIP central directory offset overflow",
        )
    })?;
    if total_entries as usize > MAX_ARCHIVE_ENTRIES || central_end > eocd {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ZIP central directory is invalid or too large",
        ));
    }

    let mut entries = Vec::with_capacity(total_entries as usize);
    let mut offset = central_offset;
    let mut total_uncompressed = 0usize;
    for _ in 0..total_entries {
        if read_u32(data, offset)? != 0x0201_4b50 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid ZIP central directory entry",
            ));
        }
        let flags = read_u16(data, offset + 8)?;
        let method = read_u16(data, offset + 10)?;
        let expected_crc = read_u32(data, offset + 16)?;
        let compressed_size = read_u32(data, offset + 20)? as usize;
        let uncompressed_size = read_u32(data, offset + 24)? as usize;
        let name_length = read_u16(data, offset + 28)? as usize;
        let extra_length = read_u16(data, offset + 30)? as usize;
        let comment_length = read_u16(data, offset + 32)? as usize;
        let external_attributes = read_u32(data, offset + 38)?;
        let local_offset = read_u32(data, offset + 42)? as usize;
        let end = offset
            .checked_add(46 + name_length + extra_length + comment_length)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "ZIP entry length overflow")
            })?;
        if end > central_end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "truncated or oversized ZIP central directory",
            ));
        }
        if flags & 0x0001 != 0 || flags & 0x0040 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "encrypted ZIP entries are not supported",
            ));
        }
        if method != 0 && method != 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported ZIP compression method {method}"),
            ));
        }
        if uncompressed_size > MAX_ENTRY_BYTES || compressed_size > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ZIP entry exceeds the supported size limit",
            ));
        }
        if uncompressed_size > 0 {
            total_uncompressed = total_uncompressed
                .checked_add(uncompressed_size)
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "ZIP expanded size overflow")
                })?;
            if total_uncompressed > MAX_EXTRACTED_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "mod expands beyond the 256 MiB limit",
                ));
            }
        }

        let name_range = checked_range(data, offset + 46, name_length)?;
        let name = std::str::from_utf8(&data[name_range.clone()]).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "ZIP entry names must be UTF-8")
        })?;
        let is_directory = name.ends_with('/');
        let clean_name = name.trim_end_matches('/');
        validate_relative_path(clean_name)?;
        let unix_mode = (external_attributes >> 16) as u16;
        if unix_mode & 0o170000 == 0o120000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "symbolic links are not allowed in mod archives",
            ));
        }

        let local_header_range = checked_range(data, local_offset, 30)?;
        if read_u32(data, local_offset)? != 0x0403_4b50 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid ZIP local file header",
            ));
        }
        let local_flags = read_u16(data, local_offset + 6)?;
        let local_method = read_u16(data, local_offset + 8)?;
        let local_name_length = read_u16(data, local_offset + 26)? as usize;
        let local_extra_length = read_u16(data, local_offset + 28)? as usize;
        if local_flags != flags || local_method != method {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ZIP local and central headers disagree",
            ));
        }
        let local_name_start = local_header_range.end;
        let local_name_range = checked_range(data, local_name_start, local_name_length)?;
        if data[local_name_range] != data[name_range] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ZIP local and central entry names disagree",
            ));
        }
        let payload_start = local_name_start
            .checked_add(local_name_length)
            .and_then(|value| value.checked_add(local_extra_length))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "ZIP payload offset overflow")
            })?;
        let payload_end = payload_start.checked_add(compressed_size).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "ZIP payload length overflow")
        })?;
        if payload_end > central_offset || payload_end > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "truncated or overlapping ZIP entry data",
            ));
        }

        let contents = if is_directory {
            Vec::new()
        } else if method == 0 {
            if compressed_size != uncompressed_size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "stored ZIP entry has inconsistent sizes",
                ));
            }
            data[payload_start..payload_end].to_vec()
        } else {
            let decoder = DeflateDecoder::new(&data[payload_start..payload_end]);
            let mut decoded = Vec::with_capacity(uncompressed_size.min(MAX_ENTRY_BYTES));
            decoder
                .take((MAX_ENTRY_BYTES + 1) as u64)
                .read_to_end(&mut decoded)?;
            decoded
        };

        if contents.len() != uncompressed_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ZIP entry has an unexpected expanded size",
            ));
        }
        if !is_directory && crc32(&contents) != expected_crc {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ZIP entry checksum mismatch",
            ));
        }
        entries.push(ArchiveEntry {
            path: clean_name.to_string(),
            is_directory,
            contents,
        });
        offset = end;
    }
    Ok(entries)
}

fn checked_range(data: &[u8], offset: usize, length: usize) -> io::Result<std::ops::Range<usize>> {
    let end = offset.checked_add(length).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "ZIP structure offset overflow")
    })?;
    if end > data.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "truncated ZIP structure",
        ));
    }
    Ok(offset..end)
}

fn read_u16(data: &[u8], offset: usize) -> io::Result<u16> {
    let range = checked_range(data, offset, 2)?;
    Ok(u16::from_le_bytes([
        data[range.start],
        data[range.start + 1],
    ]))
}

fn read_u32(data: &[u8], offset: usize) -> io::Result<u32> {
    let range = checked_range(data, offset, 4)?;
    Ok(u32::from_le_bytes([
        data[range.start],
        data[range.start + 1],
        data[range.start + 2],
        data[range.start + 3],
    ]))
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_manifest_paths_and_ids() {
        assert!(validate_relative_path("../escape.png").is_err());
        assert!(validate_relative_path("assets/../../escape.png").is_err());
        assert!(validate_relative_path("C:/escape.png").is_err());
        let manifest = ModManifest {
            id: "../escape".into(),
            name: "Bad".into(),
            version: "1.0.0".into(),
            enabled: true,
            priority: 0,
            content_file: None,
        };
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn asset_replacement_order_is_priority_then_stable_id() {
        let mut providers = vec![
            ("low".to_string(), 0),
            ("high-b".to_string(), 5),
            ("high-a".to_string(), 5),
        ];
        providers.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
        assert_eq!(
            providers.last().map(|value| value.0.as_str()),
            Some("high-b")
        );
    }

    #[test]
    fn asset_only_mods_are_valid_packages() {
        let root =
            std::env::temp_dir().join(format!("ashen_asset_only_mod_{}", std::process::id()));
        let assets = root.join("assets").join("icons");
        fs::create_dir_all(&assets).expect("test asset directories should be created");
        let manifest = ModManifest {
            id: "example.assets".into(),
            name: "Example Assets".into(),
            version: "1.0.0".into(),
            enabled: true,
            priority: 0,
            content_file: None,
        };
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&manifest).expect("manifest should serialize"),
        )
        .expect("manifest should be written");
        let mut png = vec![0u8; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&32u32.to_be_bytes());
        png[20..24].copy_from_slice(&32u32.to_be_bytes());
        fs::write(assets.join("icon.png"), png).expect("asset should be written");

        let validated = validate_staged_mod(&root).expect("asset-only mod should validate");
        assert_eq!(validated.id, "example.assets");
        fs::remove_dir_all(root).expect("test directory should be removed");
    }

    #[test]
    fn crc32_matches_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }
}
