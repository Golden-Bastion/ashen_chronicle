use super::*;

#[test]
fn gameplay_reentry_does_not_reset_pause_selection() {
    assert!(!should_reset_gameplay_selection(
        Some(ScreenId::Options),
        Some(ScreenId::Gameplay),
        GameplayScreen::Pause
    ));
}

#[test]
fn gameplay_reentry_resets_selection_for_normal_gameplay() {
    assert!(should_reset_gameplay_selection(
        Some(ScreenId::Character),
        Some(ScreenId::Gameplay),
        GameplayScreen::Dashboard
    ));
}
