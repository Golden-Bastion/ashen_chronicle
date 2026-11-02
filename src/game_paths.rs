use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const GAME_DIRECTORY_NAME: &str = "The Ashen Chronicle";
pub const SAVES_DIRECTORY_NAME: &str = "saves";
pub const MODS_DIRECTORY_NAME: &str = "mods";
pub const IMPORTS_DIRECTORY_NAME: &str = "imports";

static GAME_PATHS: OnceLock<GamePaths> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct GamePaths {
    pub root: PathBuf,
    pub saves_dir: PathBuf,
    pub mods_dir: PathBuf,
    pub imports_dir: PathBuf,
    pub save_imports_dir: PathBuf,
    pub mod_imports_dir: PathBuf,
    pub config_dir: PathBuf,
    pub mod_settings_path: PathBuf,
    pub bundled_assets_dir: PathBuf,
}

impl GamePaths {
    pub fn initialize() -> io::Result<()> {
        if GAME_PATHS.get().is_some() {
            return Ok(());
        }

        let root = resolve_internal_root()?;
        let saves_dir = root.join(SAVES_DIRECTORY_NAME);
        let mods_dir = root.join(MODS_DIRECTORY_NAME);
        let imports_dir = root.join(IMPORTS_DIRECTORY_NAME);
        let save_imports_dir = imports_dir.join("saves");
        let mod_imports_dir = imports_dir.join("mods");
        let config_dir = root.join("config");

        for directory in [
            root.as_path(),
            saves_dir.as_path(),
            mods_dir.as_path(),
            save_imports_dir.as_path(),
            mod_imports_dir.as_path(),
            config_dir.as_path(),
        ] {
            fs::create_dir_all(directory)?;
        }

        #[cfg(target_os = "android")]
        let bundled_assets_dir = root.join("bundled").join("assets");

        #[cfg(not(target_os = "android"))]
        let bundled_assets_dir = discover_bundled_assets_dir();

        let paths = Self {
            mod_settings_path: config_dir.join("mod_settings.json"),
            root,
            saves_dir,
            mods_dir,
            imports_dir,
            save_imports_dir,
            mod_imports_dir,
            config_dir,
            bundled_assets_dir,
        };

        GAME_PATHS.set(paths).map_err(|_| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "game storage paths were initialized concurrently",
            )
        })?;
        Ok(())
    }

    pub fn get() -> &'static Self {
        GAME_PATHS
            .get()
            .expect("GamePaths::initialize() must run before storage is accessed")
    }

    pub fn base_asset_path(&self, relative: &Path) -> PathBuf {
        self.bundled_assets_dir.join(relative)
    }
}

fn resolve_internal_root() -> io::Result<PathBuf> {
    #[cfg(target_os = "android")]
    {
        let app = bevy::android::ANDROID_APP
            .get()
            .ok_or_else(|| io::Error::other("Android Activity is unavailable"))?;
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
        let root = vm
            .attach_current_thread(|env| -> jni::errors::Result<Option<String>> {
                let raw_activity = app.activity_as_ptr() as jni::sys::jobject;
                let activity = unsafe {
                    env.as_cast_raw::<jni::refs::Global<jni::objects::JObject>>(&raw_activity)?
                };
                let value = env.call_method(
                    activity.as_ref(),
                    jni::jni_str!("getNativeGameRootPath"),
                    jni::jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?;
                let object = value.l()?;
                if object.as_raw().is_null() {
                    return Ok(None);
                }
                let string = jni::objects::JString::cast_local(env, object)?;
                Ok(Some(string.try_to_string(env)?))
            })
            .map_err(|error| {
                io::Error::other(format!("could not resolve Android private storage: {error}"))
            })?
            .ok_or_else(|| io::Error::other("Android Activity returned an empty storage path"))?;
        Ok(PathBuf::from(root))
    }

    #[cfg(not(target_os = "android"))]
    {
        let data_root = dirs::data_local_dir()
            .or_else(dirs::data_dir)
            .ok_or_else(|| io::Error::other("no application data directory is available"))?;
        Ok(data_root.join(GAME_DIRECTORY_NAME))
    }
}

#[cfg(not(target_os = "android"))]
fn discover_bundled_assets_dir() -> PathBuf {
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));

    if let Some(directory) = executable_dir {
        let adjacent = directory.join("data").join("assets");
        if adjacent.is_dir() {
            return adjacent;
        }
        if let Some(parent) = directory.parent() {
            let packaged = parent.join("data").join("assets");
            if packaged.is_dir() {
                return packaged;
            }
        }
        return adjacent;
    }

    PathBuf::from("data").join("assets")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_data_paths_are_grouped_under_one_root() {
        let root = PathBuf::from("/tmp/ashen-chronicle");
        assert_eq!(root.join(SAVES_DIRECTORY_NAME), root.join("saves"));
        assert_eq!(root.join(MODS_DIRECTORY_NAME), root.join("mods"));
        assert_eq!(root.join(IMPORTS_DIRECTORY_NAME), root.join("imports"));
    }
}
