use super::*;

#[test]
fn build_view_preserves_world_and_character_data() {
    let state = crate::model::create_new_state(
        "Test World",
        crate::model::WorldMode::New,
        "Ash".to_string(),
        "Wanderer".to_string(),
    );

    let view = build_view(&state);

    assert_eq!(view.world_name, state.world.name);
    assert_eq!(view.time_points, state.world.time_points);
    assert_eq!(view.day, state.world.day);
    assert_eq!(view.character.name, state.character.name);
    assert_eq!(view.character.title, state.character.title);
    assert_eq!(view.character.hp, state.character.hp);
    assert_eq!(view.character.max_hp, state.character.max_hp);
    assert!(view.location.is_some());
}
