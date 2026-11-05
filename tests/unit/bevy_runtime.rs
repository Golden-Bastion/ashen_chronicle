use super::*;

#[test]
fn default_runtime_state_is_active() {
    let state = RuntimeLifecycleState::default();
    assert_eq!(state.app_lifecycle, AppLifecycle::Running);
    assert!(state.window_focused);
}
