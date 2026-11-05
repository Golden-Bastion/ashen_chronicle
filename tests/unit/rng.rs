use super::*;

#[test]
fn identical_seeds_produce_identical_sequences() {
    let mut a = DeterministicRng::new(123);
    let mut b = DeterministicRng::new(123);
    assert_eq!(a.next_u64(), b.next_u64());
    assert_eq!(a.next_bool(), b.next_bool());
    assert_eq!(a.gen_range(2..10), b.gen_range(2..10));
}

#[test]
fn state_round_trip_continues_the_same_sequence() {
    let mut original = DeterministicRng::new(456);
    let _ = original.next_u64();
    let saved = original.state();
    let mut restored = DeterministicRng::from_state(saved);
    assert_eq!(original.next_u64(), restored.next_u64());
}
