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
