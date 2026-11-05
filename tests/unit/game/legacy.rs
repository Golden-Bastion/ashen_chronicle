use super::*;

#[test]
fn remains_search_advances_character_turn_with_time() {
    let mut state = crate::model::create_new_state(
        "Test World",
        crate::model::WorldMode::New,
        "Ash".to_string(),
        "Wanderer".to_string(),
    );
    let location_id = state.character.location_id;
    let corpse_id = state.world.allocate_id();
    state.corpses.push(crate::model::Corpse {
        id: corpse_id,
        location_id,
        inventory: vec![crate::model::Item {
            id: state.world.allocate_id(),
            name: "Relic".to_string(),
            description: "A weathered relic.".to_string(),
        }],
        former_name: "Mara".to_string(),
        former_title: "Keeper".to_string(),
        ..Default::default()
    });

    let old_turn = state.character.turn;
    let old_time_points = state.world.time_points;
    let result = search_remains_for_bevy(&mut state, 0);

    assert!(result.is_ok());
    assert_eq!(state.character.turn, old_turn + 1);
    assert_eq!(state.world.time_points, (old_time_points + 1) % 12);
}

#[test]
fn negative_insight_experience_is_clamped_instead_of_panicking() {
    assert_eq!(remains_experience(-6), 0);
    assert_eq!(remains_experience(0), 5);
    assert_eq!(remains_experience(3), 8);
}
