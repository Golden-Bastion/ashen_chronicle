use super::*;
use crate::model::{create_new_state, WorldMode};

#[test]
fn initial_snapshot_does_not_create_feedback() {
    let state = create_new_state(
        "Test World",
        WorldMode::New,
        "Ash".to_string(),
        "Wanderer".to_string(),
    );
    let mut feedback = FeedbackState::default();
    snapshot_state(&mut feedback, &state);
    assert_eq!(feedback.last_level, state.character.level);
    assert!(feedback.entries.is_empty());
}

#[test]
fn quest_reward_context_is_only_conversation() {
    assert!(is_quest_reward_context(Some(ScreenId::Conversation)));
    assert!(!is_quest_reward_context(Some(ScreenId::Gameplay)));
}

#[test]
fn selection_wraps_across_three_attributes() {
    let mut feedback = FeedbackState {
        entries: VecDeque::from([FeedbackEntry::LevelUp { level: 2 }]),
        ..Default::default()
    };
    move_level_selection(&mut feedback, -1);
    assert_eq!(feedback.selected, 2);
}
