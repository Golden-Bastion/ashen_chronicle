use super::*;

#[test]
fn developer_console_long_press_uses_a_deliberate_threshold() {
    assert!(!should_open_developer_console(10.64, 10.0, false));
    assert!(should_open_developer_console(10.66, 10.0, false));
    assert!(!should_open_developer_console(10.80, 10.0, true));
}
