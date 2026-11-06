use super::*;

#[test]
fn character_tabs_have_a_stable_five_tab_order() {
    assert_eq!(
        CharacterTab::ALL.map(CharacterTab::index),
        [0, 1, 2, 3, 4]
    );
    assert_eq!(CharacterTab::from_index(0), Some(CharacterTab::General));
    assert_eq!(CharacterTab::from_index(4), Some(CharacterTab::Journal));
    assert_eq!(CharacterTab::from_index(5), None);
    assert_eq!(CharacterTab::Journal.next(), CharacterTab::General);
}

#[test]
fn journal_does_not_repeat_the_note_as_a_chronicle_entry() {
    let notes = vec!["I should remember this.".to_string()];

    assert!(is_personal_note_history_entry(
        "Ada noted: I should remember this.",
        "Ada",
        &notes
    ));
    assert!(!is_personal_note_history_entry(
        "Ada noted: a different note.",
        "Ada",
        &notes
    ));
    assert!(!is_personal_note_history_entry(
        "Ada found a sealed letter.",
        "Ada",
        &notes
    ));
}
