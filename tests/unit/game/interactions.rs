use super::*;
use crate::model::{create_new_state, WorldMode};

#[test]
fn conversation_choice_advances_character_turn() {
    let mut state = create_new_state(
        "Test World",
        WorldMode::New,
        "Ash".to_string(),
        "Wanderer".to_string(),
    );
    let location_id = state.character.location_id;
    let npc_id = state.world.allocate_id();
    let mut npc = crate::model::Npc::new(npc_id, "Mara", "Keeper", location_id, None);
    npc.memory.push("The road remembers.".to_string());
    state.npcs.push(npc);
    state.character.attributes.insight = 2;

    assert_eq!(state.character.turn, 0);
    let messages = perform_conversation_choice(&mut state, npc_id, 2);

    assert_eq!(state.character.turn, 1);
    assert!(messages
        .iter()
        .any(|message| message.contains("The road remembers.")));
}

#[test]
fn conversation_memory_skips_internal_metadata_and_uses_the_latest_player_memory() {
    let mut state = create_new_state(
        "Test World",
        WorldMode::New,
        "Ash".to_string(),
        "Wanderer".to_string(),
    );
    let npc_id = state.world.allocate_id();
    let mut npc = crate::model::Npc::new(
        npc_id,
        "Mara",
        "Keeper",
        state.character.location_id,
        None,
    );
    npc.memory.push("The road remembers.".to_string());
    npc.memory
        .push("[authored anchor] Mara operates at the old road.".to_string());
    let npc_index = state.npcs.len();
    state.npcs.push(npc);
    state.character.attributes.insight = 2;

    let view = build_conversation_view(&state, npc_index);

    assert_eq!(view.memory.as_deref(), Some("The road remembers."));
    assert!(view.options.iter().any(|option| option == "Ask what they remember"));
}

#[test]
fn conversation_does_not_offer_internal_metadata_as_a_remembered_event() {
    let mut state = create_new_state(
        "Test World",
        WorldMode::New,
        "Ash".to_string(),
        "Wanderer".to_string(),
    );
    let mut npc = crate::model::Npc::new(
        state.world.allocate_id(),
        "Mara",
        "Keeper",
        state.character.location_id,
        None,
    );
    npc.memory
        .push("[authored anchor] Mara operates at the old road.".to_string());
    state.npcs.push(npc);
    state.character.attributes.insight = 2;

    let view = build_conversation_view(&state, 0);

    assert_eq!(view.memory, None);
    assert!(!view.options.iter().any(|option| option == "Ask what they remember"));
}
