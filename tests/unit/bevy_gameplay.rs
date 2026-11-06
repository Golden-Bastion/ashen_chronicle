use super::*;

#[test]
fn gameplay_runtime_pause_entry_and_exit_update_runtime_state() {
    let mut gameplay = GameplayState::default();
    let mut runtime = GameplayRuntimeState::default();
    let mut navigation = NavigationState::default();

    open_pause(&mut gameplay, &mut runtime, &mut navigation);
    assert_eq!(gameplay.screen, GameplayScreen::Pause);
    assert!(runtime.is_paused());

    close_pause(&mut gameplay, &mut runtime, &mut navigation);
    assert_eq!(gameplay.screen, GameplayScreen::Dashboard);
    assert!(!runtime.is_paused());
}

#[test]
fn animate_world_clouds_does_not_move_clouds_while_paused() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.insert_resource(GameplayRuntimeState { paused: true });

    let entity = app
        .world_mut()
        .spawn((
            WorldCloud {
                base_x: 42.0,
                drift: 8.0,
                speed: 1.0,
            },
            Node {
                left: percent(17.0),
                ..default()
            },
        ))
        .id();

    app.add_systems(Update, animate_world_clouds);
    app.update();

    let node = app
        .world()
        .get::<Node>(entity)
        .expect("cloud node should remain");
    assert_eq!(node.left, percent(17.0));
}

#[test]
fn pause_action_count_matches_load_visibility() {
    assert_eq!(pause_actions_from(false).len(), 5);
    assert_eq!(pause_actions_from(true).len(), 6);
}

#[test]
fn pause_action_order_keeps_options_before_quit() {
    assert!(matches!(
        pause_actions_from(true).as_slice(),
        [
            PauseAction::Resume,
            PauseAction::Character,
            PauseAction::NewGame,
            PauseAction::LoadGame,
            PauseAction::Options,
            PauseAction::Quit
        ]
    ));
}
