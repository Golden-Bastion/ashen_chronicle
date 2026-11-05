use super::{CharacterView, HistoryView, InventoryView, MeditationView, NpcView, WorldView};

#[test]
fn display_name_includes_nonempty_title_only() {
    let character = CharacterView {
        name: "Ash".to_string(),
        title: "Wanderer".to_string(),
        ..Default::default()
    };
    let npc = NpcView {
        name: "Mara".to_string(),
        title: "Keeper".to_string(),
        ..Default::default()
    };
    let untitled_character = CharacterView {
        name: "Ash".to_string(),
        ..Default::default()
    };
    let untitled_npc = NpcView {
        name: "Mara".to_string(),
        ..Default::default()
    };

    assert_eq!(character.display_name(), "Ash the Wanderer");
    assert_eq!(npc.display_name(), "Mara the Keeper");
    assert_eq!(untitled_character.display_name(), "Ash");
    assert_eq!(untitled_npc.display_name(), "Mara");
}

#[test]
fn empty_screen_views_are_frontend_safe() {
    let world = WorldView::default();
    let history = HistoryView::default();
    let inventory = InventoryView::default();
    let meditation = MeditationView::default();

    assert!(world.world_name.is_empty());
    assert!(world.time.is_empty());
    assert!(world.location.is_none());
    assert!(world.threat.is_none());
    assert!(world.history.is_empty());
    assert!(history.world_name.is_empty());
    assert!(history.entries.is_empty());
    assert!(inventory.items.is_empty());
    assert!(meditation.targets.is_empty());
}
