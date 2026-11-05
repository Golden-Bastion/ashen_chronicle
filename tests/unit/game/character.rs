use super::build_character_sheet_view;
use crate::model::{create_new_state, Condition, Faction, WorldMode};

#[test]
fn character_sheet_view_preserves_derived_character_data() {
    let mut state = create_new_state(
        "Test World",
        WorldMode::New,
        "Ash".to_string(),
        "Wanderer".to_string(),
    );
    state.character.level = 3;
    state.character.experience = 27;
    state.character.hp = 8;
    state.character.attributes.might = 4;
    state.character.conditions = vec![Condition {
        name: "Wounded".to_string(),
        remaining: 2,
        penalty: -1,
        bonus: 0,
    }];
    let mut faction = Faction::new(20, "Wardens");
    faction.reputation = 5;
    faction.memory.push("A debt remembered.".to_string());
    state.factions.push(faction);

    let view = build_character_sheet_view(&state);

    assert_eq!(view.character.display_name(), "Ash the Wanderer");
    assert_eq!(view.level, 3);
    assert_eq!(view.experience, 27);
    assert_eq!(view.next_level_experience, 150);
    assert_eq!(view.attributes.might, 4);
    assert_eq!(view.attributes.effective_might, 3);
    assert_eq!(view.conditions[0].remaining, 2);
    assert_eq!(view.factions[0].reputation, 5);
    assert_eq!(view.factions[0].memories, vec!["A debt remembered."]);
}
