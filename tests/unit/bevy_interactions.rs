use super::*;

#[test]
fn interaction_screen_set_matches_navigation_targets() {
    assert!(is_interaction_screen(Some(ScreenId::Talk)));
    assert!(is_interaction_screen(Some(ScreenId::Conversation)));
    assert!(is_interaction_screen(Some(ScreenId::Remains)));
    assert!(is_interaction_screen(Some(ScreenId::RemainsResult)));
    assert!(!is_interaction_screen(Some(ScreenId::Gameplay)));
}
