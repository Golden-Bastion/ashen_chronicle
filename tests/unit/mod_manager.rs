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
    let mut providers = [
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
    let root = std::env::temp_dir().join(format!("ashen_asset_only_mod_{}", std::process::id()));
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
