use super::*;

#[test]
fn bundled_mod_seeding_does_not_overwrite_user_files() {
    let root = std::env::temp_dir().join(format!(
        "ever_unfolding_saga_seed_test_{}",
        std::process::id()
    ));
    let source = root.join("source");
    let target = root.join("target");
    fs::create_dir_all(source.join("example")).unwrap();
    fs::create_dir_all(target.join("example")).unwrap();
    fs::write(source.join("example/manifest.json"), "bundled").unwrap();
    fs::write(target.join("example/manifest.json"), "user").unwrap();
    copy_directory_without_overwriting(&source, &target).unwrap();
    assert_eq!(
        fs::read_to_string(target.join("example/manifest.json")).unwrap(),
        "user"
    );
    fs::remove_dir_all(root).unwrap();
}
