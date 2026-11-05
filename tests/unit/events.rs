use super::*;
use crate::content::EventConditionContent;
use crate::model::{create_new_state, WorldMode};

fn test_state() -> GameState {
    create_new_state(
        "Tester",
        WorldMode::New,
        "Ash Walker".into(),
        "the Test Subject".into(),
    )
}

#[test]
fn night_condition_matches_expected_context() {
    let state = test_state();
    let condition = EventConditionContent {
        night: Some(true),
        ..Default::default()
    };
    let day = EventContext::for_travel_arrival("Ashen Gate", false, false);
    let night = EventContext::for_travel_arrival("Ashen Gate", false, true);
    assert!(!matches_conditions(Some(&condition), &state, &day));
    assert!(matches_conditions(Some(&condition), &state, &night));
}

#[test]
fn reputation_condition_matches_current_faction_standing() {
    let mut state = test_state();
    state
        .factions
        .push(crate::model::Faction::new(99, "Cinder Wardens"));
    state.factions[0].reputation = 5;
    let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
    let condition = EventConditionContent {
        faction_name: Some("Cinder Wardens".into()),
        min_reputation: Some(5),
        max_reputation: Some(10),
        ..Default::default()
    };
    assert!(matches_conditions(Some(&condition), &state, &context));
    state.factions[0].reputation = 4;
    assert!(!matches_conditions(Some(&condition), &state, &context));
}

#[test]
fn inventory_and_condition_requirements_are_enforced() {
    let mut state = test_state();
    let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
    let condition = EventConditionContent {
        required_item_name: Some("Old Key".into()),
        required_condition_name: Some("Exhausted".into()),
        ..Default::default()
    };
    assert!(!matches_conditions(Some(&condition), &state, &context));
    state.character.inventory.push(crate::model::Item {
        id: 77,
        name: "Old Key".into(),
        description: "A rusted key.".into(),
    });
    state
        .character
        .conditions
        .push(crate::model::Condition::new("Exhausted", 2, -1));
    assert!(matches_conditions(Some(&condition), &state, &context));
}

#[test]
fn reputation_condition_defaults_missing_faction_to_ineligible() {
    let state = test_state();
    let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
    let condition = EventConditionContent {
        min_reputation: Some(1),
        ..Default::default()
    };
    assert!(!matches_conditions(Some(&condition), &state, &context));
}

#[test]
fn prior_event_condition_uses_structured_history() {
    let mut state = test_state();
    let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
    let condition = EventConditionContent {
        prior_event_id: Some("event.previous".into()),
        ..Default::default()
    };
    assert!(!matches_conditions(Some(&condition), &state, &context));
    state
        .world
        .record_event_history(0, "event.previous", "Ashen Gate", "The omen appeared.");
    assert!(matches_conditions(Some(&condition), &state, &context));
}

#[test]
fn applied_event_records_structured_history() {
    let mut state = test_state();
    let event = EventContent {
        id: "event.test".into(),
        trigger: "travel_arrival".into(),
        weight: 1,
        chance_percent: Some(100),
        cooldown_turns: None,
        conditions: None,
        effects: vec![EventEffectContent::History {
            text: "A sign appears.".into(),
        }],
    };
    let context = EventContext::for_travel_arrival("Ashen Gate", false, false);
    apply_event(&mut state, &event, &context);
    let entry = state
        .world
        .history
        .last()
        .expect("event history should exist");
    assert_eq!(entry.entry_type, crate::model::HistoryEntryType::Event);
    assert_eq!(entry.event_id.as_deref(), Some("event.test"));
    assert_eq!(entry.location_name.as_deref(), Some("Ashen Gate"));
    assert_eq!(entry.text, "A sign appears.");
    assert!(entry
        .outcome
        .as_deref()
        .unwrap()
        .contains("A sign appears."));
}

#[test]
fn event_rng_sequence_is_reproducible_from_persisted_state() {
    let mut a = test_state();
    let mut b = a.clone();
    let first_a = next_random_roll(&mut a);
    let first_b = next_random_roll(&mut b);
    let second_a = next_random_roll(&mut a);
    let second_b = next_random_roll(&mut b);
    assert_eq!((first_a, second_a), (first_b, second_b));
    assert_ne!(a.rng_state, test_state().rng_state);
}

#[test]
fn cooldown_blocks_event_until_turn_is_reached() {
    let mut state = test_state();
    state
        .world
        .event_cooldowns
        .push(crate::model::EventCooldown {
            event_id: "test.event".into(),
            ready_at_turn: 3,
        });
    state.character.turn = 2;
    assert!(!event_is_off_cooldown(&state, "test.event"));
    state.character.turn = 3;
    assert!(event_is_off_cooldown(&state, "test.event"));
}

#[test]
fn add_condition_event_refreshes_existing_condition() {
    let mut state = test_state();
    state
        .character
        .conditions
        .push(Condition::new("Wounded", 2, -1));
    let event = EventContent {
        id: "event.condition".into(),
        trigger: "travel_arrival".into(),
        weight: 1,
        chance_percent: Some(100),
        cooldown_turns: None,
        conditions: None,
        effects: vec![EventEffectContent::AddCondition {
            name: "Wounded".into(),
            remaining: 5,
            penalty: -2,
            bonus: 1,
        }],
    };
    let context = EventContext::for_travel_arrival("Ashen Gate", false, false);

    apply_event(&mut state, &event, &context);

    assert_eq!(state.character.conditions.len(), 1);
    let condition = &state.character.conditions[0];
    assert_eq!(condition.name, "Wounded");
    assert_eq!(condition.remaining, 5);
    assert_eq!(condition.penalty, -2);
    assert_eq!(condition.bonus, 1);
}
