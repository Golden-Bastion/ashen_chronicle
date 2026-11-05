use super::*;

#[test]
fn canonical_app_icon_decodes() {
    assert!(load_window_icon().is_some());
}
