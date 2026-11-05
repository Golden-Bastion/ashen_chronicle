use super::*;

#[test]
fn options_tabs_have_three_categories() {
    let mut options = OptionsState::default();
    assert_eq!(options_selection_end(&options), 1); // Import and Back.
    options.tab = 1;
    assert_eq!(options_selection_end(&options), 1); // Install and Back.
    options.tab = 2;
    assert_eq!(options_selection_end(&options), 5);
}

#[test]
fn mod_action_slots_match_the_row_control_order() {
    let second_mod_first_action = mod_action_start_index(1);
    assert_eq!(MOD_ENABLE_ACTION, 0);
    assert_eq!(MOD_DELETE_ACTION, 1);
    assert_eq!(MOD_PRIORITY_UP_ACTION, 2);
    assert_eq!(MOD_PRIORITY_DOWN_ACTION, 3);
    assert_eq!(second_mod_first_action, 4);
    assert_eq!(mod_install_action_index(2), 8);
    assert_eq!(mod_back_action_index(2), 9);
    assert_eq!(
        second_mod_first_action + MOD_PRIORITY_DOWN_ACTION,
        second_mod_first_action + MOD_ACTIONS_PER_MOD - 1
    );
}
