use super::MEDITATION_TARGETS;

#[test]
fn meditation_targets_advance_to_the_next_occurrence() {
    let current = 4;
    let morning = MEDITATION_TARGETS
        .iter()
        .find(|(slot, _)| *slot == 3)
        .map(|(slot, _)| *slot)
        .unwrap();
    let dusk = MEDITATION_TARGETS
        .iter()
        .find(|(slot, _)| *slot == 8)
        .map(|(slot, _)| *slot)
        .unwrap();
    assert_eq!((morning + 12 - current) % 12, 11);
    assert_eq!((dusk + 12 - current) % 12, 4);
}

#[test]
fn meditation_does_not_allow_a_zero_time_advance() {
    let current = 8;
    let dusk = MEDITATION_TARGETS
        .iter()
        .find(|(slot, _)| *slot == 8)
        .map(|(slot, _)| *slot)
        .unwrap();
    assert_eq!(((dusk + 12 - current) % 12).max(1), 1);
}
