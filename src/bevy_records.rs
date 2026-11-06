//! Bevy presentation and navigation for non-combat gameplay records.
//!
//! Character, inventory, quest, meditation, history, and journal screens reuse
//! the existing frontend-neutral view models and authoritative game mutations.

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::bevy_lifecycle::{LifecyclePhase, LifecycleState};
use crate::bevy_presentation::{
    self, BevyScreenRoot, GameplayInputQueue, NavigationState, ScreenId,
};
use crate::game::{actions, character, history_screen, records};
use crate::input::InputEvent;
use crate::presentation::{
    CharacterSheetView, HistoryEntryView, HistoryView, InventoryDetailView, InventoryView,
    QuestLogView, QuestView,
};

const CHARACTER_TAB_LABELS: [&str; 5] = ["General", "Reputation", "Inventory", "Quests", "Journal"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharacterTab {
    General,
    Reputation,
    Inventory,
    Quests,
    Journal,
}

impl CharacterTab {
    const ALL: [Self; 5] = [
        Self::General,
        Self::Reputation,
        Self::Inventory,
        Self::Quests,
        Self::Journal,
    ];

    fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    fn index(self) -> usize {
        match self {
            Self::General => 0,
            Self::Reputation => 1,
            Self::Inventory => 2,
            Self::Quests => 3,
            Self::Journal => 4,
        }
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordScreen {
    Character,
    InventoryDetail,
    QuestDetail,
    Meditation,
    MeditationResult,
    ChronicleDetail,
    JournalEntry,
}

#[derive(Resource)]
pub(crate) struct RecordScreenState {
    screen: RecordScreen,
    active_navigation: Option<ScreenId>,
    character_tab: CharacterTab,
    selected: usize,
    detail_index: usize,
    message: Option<String>,
    meditation_result: Option<crate::presentation::MeditationResultView>,
    journal_draft: String,
    dirty: bool,
}

impl Default for RecordScreenState {
    fn default() -> Self {
        Self {
            screen: RecordScreen::Character,
            active_navigation: None,
            character_tab: CharacterTab::General,
            selected: 0,
            detail_index: 0,
            message: None,
            meditation_result: None,
            journal_draft: String::new(),
            dirty: true,
        }
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<RecordScreenState>().add_systems(
        Update,
        (sync_screen, text_input, record_input, render_if_active).chain(),
    );
}

fn is_record_screen(screen: Option<ScreenId>) -> bool {
    matches!(screen, Some(ScreenId::Character | ScreenId::Meditation))
}

fn sync_screen(
    lifecycle: Res<LifecycleState>,
    navigation: Res<NavigationState>,
    mut records_state: ResMut<RecordScreenState>,
) {
    if lifecycle.phase != LifecyclePhase::Complete || !is_record_screen(navigation.current_screen) {
        records_state.active_navigation = None;
        return;
    }

    let Some(screen) = navigation.current_screen else {
        return;
    };
    if records_state.active_navigation == Some(screen) {
        return;
    }

    records_state.active_navigation = Some(screen);
    records_state.screen = match screen {
        ScreenId::Character => {
            records_state.character_tab = CharacterTab::General;
            RecordScreen::Character
        }
        ScreenId::Meditation => RecordScreen::Meditation,
        _ => return,
    };
    records_state.selected = 0;
    records_state.detail_index = 0;
    records_state.message = None;
    records_state.meditation_result = None;
    records_state.journal_draft.clear();
    records_state.dirty = true;
}

fn text_input(
    mut keyboard: MessageReader<KeyboardInput>,
    navigation: Res<NavigationState>,
    mut records_state: ResMut<RecordScreenState>,
) {
    if !is_record_screen(navigation.current_screen)
        || records_state.screen != RecordScreen::JournalEntry
        || records_state.message.is_some()
    {
        return;
    }

    for event in keyboard.read() {
        if let Some(text) = &event.text {
            if !text.chars().any(char::is_control) {
                records_state.journal_draft.push_str(text);
                records_state.dirty = true;
            }
        }
    }
}

fn record_input(
    mut lifecycle: ResMut<LifecycleState>,
    mut navigation: ResMut<NavigationState>,
    mut records_state: ResMut<RecordScreenState>,
    mut input_queue: ResMut<GameplayInputQueue>,
) {
    if lifecycle.phase != LifecyclePhase::Complete
        || !is_record_screen(navigation.current_screen)
        || input_queue.0.is_empty()
    {
        return;
    }

    let events = std::mem::take(&mut input_queue.0);
    for event in events {
        match event {
            InputEvent::SelectTab(index) if records_state.screen == RecordScreen::Character => {
                if let Some(tab) = CharacterTab::from_index(index) {
                    records_state.character_tab = tab;
                    records_state.selected = 0;
                    records_state.message = None;
                    records_state.dirty = true;
                }
            }
            InputEvent::Tab if records_state.screen == RecordScreen::Character => {
                records_state.character_tab = records_state.character_tab.next();
                records_state.selected = 0;
                records_state.message = None;
                records_state.dirty = true;
            }
            InputEvent::Up => move_selection(&mut records_state, &lifecycle, -1),
            InputEvent::Down => move_selection(&mut records_state, &lifecycle, 1),
            InputEvent::Home => {
                records_state.selected = 0;
                records_state.dirty = true;
            }
            InputEvent::End => move_to_end(&mut records_state, &lifecycle),
            InputEvent::PageUp => move_selection(&mut records_state, &lifecycle, -5),
            InputEvent::PageDown => move_selection(&mut records_state, &lifecycle, 5),
            InputEvent::Backspace => {
                if records_state.screen == RecordScreen::JournalEntry
                    && records_state.message.is_none()
                {
                    records_state.journal_draft.pop();
                    records_state.dirty = true;
                }
            }
            InputEvent::Confirm => {
                records_state.selected = navigation.selected;
                activate(&mut lifecycle, &mut navigation, &mut records_state);
            }
            InputEvent::Cancel => cancel(&mut navigation, &mut records_state),
            _ => {}
        }
        navigation.selected = records_state.selected;
    }
}

fn journal_chronicle_entries<'a>(
    history: &'a HistoryView,
    character: &CharacterSheetView,
) -> Vec<(usize, &'a HistoryEntryView)> {
    history
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            !is_personal_note_history_entry(
                &entry.text,
                &character.character.display_name(),
                &character.notes,
            )
        })
        .collect()
}

fn is_personal_note_history_entry(text: &str, character_name: &str, notes: &[String]) -> bool {
    let prefix = format!("{character_name} noted: ");
    text.strip_prefix(&prefix)
        .is_some_and(|note| notes.iter().any(|saved_note| saved_note == note))
}

fn selection_count(records_state: &RecordScreenState, lifecycle: &LifecycleState) -> usize {
    let Some(session) = lifecycle.session.as_ref() else {
        return 1;
    };
    match records_state.screen {
        RecordScreen::Character => match records_state.character_tab {
            CharacterTab::General | CharacterTab::Reputation => 0,
            CharacterTab::Inventory => records::build_inventory_view(&session.state).items.len(),
            CharacterTab::Quests => records::build_quest_log_view(&session.state).quests.len(),
            CharacterTab::Journal => {
                let character_view = character::build_character_sheet_view(&session.state);
                let history_view = history_screen::build_view(&session.state);
                journal_chronicle_entries(&history_view, &character_view).len() + 1
            }
        },
        RecordScreen::InventoryDetail
        | RecordScreen::QuestDetail
        | RecordScreen::MeditationResult
        | RecordScreen::ChronicleDetail => 1,
        RecordScreen::Meditation => {
            let view = actions::build_meditation_view(&session.state);
            if view.safe_to_meditate {
                view.targets.len() + 1
            } else {
                1
            }
        }
        RecordScreen::JournalEntry => 2,
    }
}

fn move_selection(
    records_state: &mut RecordScreenState,
    lifecycle: &LifecycleState,
    direction: isize,
) {
    let count = selection_count(records_state, lifecycle);
    if count == 0 {
        return;
    }
    records_state.selected =
        (records_state.selected as isize + direction).rem_euclid(count as isize) as usize;
    records_state.dirty = true;
}

fn move_to_end(records_state: &mut RecordScreenState, lifecycle: &LifecycleState) {
    let count = selection_count(records_state, lifecycle);
    if count > 0 {
        records_state.selected = count - 1;
        records_state.dirty = true;
    }
}

fn close_to_gameplay(navigation: &mut NavigationState) {
    navigation.current_screen = Some(ScreenId::Gameplay);
    navigation.return_screen = None;
    navigation.selected = 0;
}

fn return_to_character(records_state: &mut RecordScreenState, tab: CharacterTab) {
    records_state.screen = RecordScreen::Character;
    records_state.character_tab = tab;
    records_state.selected = records_state.detail_index;
    records_state.message = None;
    records_state.dirty = true;
}

fn activate(
    lifecycle: &mut LifecycleState,
    navigation: &mut NavigationState,
    records_state: &mut RecordScreenState,
) {
    let Some(session) = lifecycle.session.as_mut() else {
        return;
    };

    match records_state.screen {
        RecordScreen::Character => match records_state.character_tab {
            CharacterTab::General | CharacterTab::Reputation => {}
            CharacterTab::Inventory => {
                let view = records::build_inventory_view(&session.state);
                if records_state.selected < view.items.len() {
                    records_state.detail_index = records_state.selected;
                    records_state.screen = RecordScreen::InventoryDetail;
                    records_state.selected = 0;
                    records_state.dirty = true;
                }
            }
            CharacterTab::Quests => {
                let view = records::build_quest_log_view(&session.state);
                if records_state.selected < view.quests.len() {
                    records_state.detail_index = records_state.selected;
                    records_state.screen = RecordScreen::QuestDetail;
                    records_state.selected = 0;
                    records_state.dirty = true;
                }
            }
            CharacterTab::Journal => {
                let character_view = character::build_character_sheet_view(&session.state);
                let history_view = history_screen::build_view(&session.state);
                let entries = journal_chronicle_entries(&history_view, &character_view);
                if records_state.selected < entries.len() {
                    records_state.detail_index = records_state.selected;
                    records_state.screen = RecordScreen::ChronicleDetail;
                    records_state.selected = 0;
                    records_state.dirty = true;
                } else if records_state.selected == entries.len() {
                    records_state.screen = RecordScreen::JournalEntry;
                    records_state.selected = 0;
                    records_state.message = None;
                    records_state.journal_draft.clear();
                    records_state.dirty = true;
                }
            }
        },
        RecordScreen::InventoryDetail => {
            return_to_character(records_state, CharacterTab::Inventory);
        }
        RecordScreen::QuestDetail => {
            return_to_character(records_state, CharacterTab::Quests);
        }
        RecordScreen::Meditation => {
            let view = actions::build_meditation_view(&session.state);
            if !view.safe_to_meditate || records_state.selected >= view.targets.len() {
                close_to_gameplay(navigation);
            } else {
                match actions::meditate_to_target(
                    &mut session.state,
                    &session.save_path,
                    records_state.selected,
                ) {
                    Ok(result) => {
                        records_state.meditation_result = Some(result);
                        records_state.screen = RecordScreen::MeditationResult;
                        records_state.selected = 0;
                        records_state.message = None;
                        records_state.dirty = true;
                    }
                    Err(err) => {
                        records_state.message = Some(err.to_string());
                        records_state.dirty = true;
                    }
                }
            }
        }
        RecordScreen::MeditationResult => close_to_gameplay(navigation),
        RecordScreen::ChronicleDetail => {
            return_to_character(records_state, CharacterTab::Journal);
        }
        RecordScreen::JournalEntry => {
            if records_state.message.is_some() {
                records_state.screen = RecordScreen::Character;
                records_state.character_tab = CharacterTab::Journal;
                records_state.selected = 0;
                records_state.message = None;
                records_state.journal_draft.clear();
                records_state.dirty = true;
                return;
            }
            match records_state.selected {
                0 => {
                    if records::record_journal_note(
                        &mut session.state,
                        &records_state.journal_draft,
                    ) {
                        records_state.message = Some("The journal entry is recorded.".to_string());
                    } else {
                        records_state.message =
                            Some("Write a note before recording it.".to_string());
                    }
                    records_state.dirty = true;
                }
                _ => {
                    records_state.screen = RecordScreen::Character;
                    records_state.character_tab = CharacterTab::Journal;
                    records_state.selected = 0;
                    records_state.journal_draft.clear();
                    records_state.message = None;
                    records_state.dirty = true;
                }
            }
        }
    }
}

fn cancel(navigation: &mut NavigationState, records_state: &mut RecordScreenState) {
    match records_state.screen {
        RecordScreen::Character | RecordScreen::Meditation | RecordScreen::MeditationResult => {
            close_to_gameplay(navigation);
        }
        RecordScreen::InventoryDetail => {
            return_to_character(records_state, CharacterTab::Inventory);
        }
        RecordScreen::QuestDetail => {
            return_to_character(records_state, CharacterTab::Quests);
        }
        RecordScreen::ChronicleDetail => {
            return_to_character(records_state, CharacterTab::Journal);
        }
        RecordScreen::JournalEntry => {
            records_state.screen = RecordScreen::Character;
            records_state.character_tab = CharacterTab::Journal;
            records_state.selected = 0;
            records_state.message = None;
            records_state.journal_draft.clear();
            records_state.dirty = true;
        }
    }
}

fn render_if_active(
    mut commands: Commands,
    lifecycle: Res<LifecycleState>,
    navigation: Res<NavigationState>,
    mut records_state: ResMut<RecordScreenState>,
    roots: Query<Entity, With<BevyScreenRoot>>,
) {
    if lifecycle.phase != LifecyclePhase::Complete || !is_record_screen(navigation.current_screen) {
        return;
    }
    if !records_state.dirty {
        return;
    }

    for root in &roots {
        commands.entity(root).despawn();
    }

    let Some(session) = lifecycle.session.as_ref() else {
        return;
    };
    let character_view = character::build_character_sheet_view(&session.state);
    match records_state.screen {
        RecordScreen::Character => render_character_hub(
            &mut commands,
            &character_view,
            &records::build_inventory_view(&session.state),
            &records::build_quest_log_view(&session.state),
            &history_screen::build_view(&session.state),
            records_state.character_tab,
            records_state.selected,
        ),
        RecordScreen::InventoryDetail => render_inventory_detail(
            &mut commands,
            &records::build_inventory_detail_view(&session.state, records_state.detail_index),
        ),
        RecordScreen::QuestDetail => render_quest_detail(
            &mut commands,
            records::build_quest_view_for(&session.state, records_state.detail_index).as_ref(),
        ),
        RecordScreen::Meditation => render_meditation(
            &mut commands,
            &actions::build_meditation_view(&session.state),
            records_state.selected,
            records_state.message.as_deref(),
        ),
        RecordScreen::MeditationResult => {
            render_meditation_result(&mut commands, records_state.meditation_result.as_ref())
        }
        RecordScreen::ChronicleDetail => {
            let history_view = history_screen::build_view(&session.state);
            let entries = journal_chronicle_entries(&history_view, &character_view);
            render_chronicle_detail(
                &mut commands,
                entries
                    .get(records_state.detail_index)
                    .map(|(_, entry)| *entry),
            );
        }
        RecordScreen::JournalEntry => render_journal_entry(
            &mut commands,
            &session.state,
            &records_state.journal_draft,
            records_state.selected,
            records_state.message.as_deref(),
        ),
    }
    records_state.dirty = false;
}

fn render_header(commands: &mut Commands, title: &str, subtitle: impl Into<String>) -> Entity {
    let root = bevy_presentation::spawn_screen(commands, title);
    let panel = bevy_presentation::spawn_panel(commands, root);
    bevy_presentation::spawn_muted_label(commands, panel, subtitle);
    panel
}

fn render_character_hub(
    commands: &mut Commands,
    character: &CharacterSheetView,
    inventory: &InventoryView,
    quests: &QuestLogView,
    history: &HistoryView,
    tab: CharacterTab,
    selected: usize,
) {
    let root = bevy_presentation::spawn_screen(commands, "CHARACTER");
    let panel = bevy_presentation::spawn_fixed_panel(commands, root);
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        format!("{} | Level {}", character.character.display_name(), character.level),
    );
    bevy_presentation::spawn_tab_strip(
        commands,
        panel,
        &CHARACTER_TAB_LABELS,
        tab.index(),
    );
    let content = bevy_presentation::spawn_scrollable_list(commands, panel);

    match tab {
        CharacterTab::General => render_character_general_tab(commands, content, character),
        CharacterTab::Reputation => render_character_reputation_tab(commands, content, character),
        CharacterTab::Inventory => render_inventory_tab(commands, content, inventory, selected),
        CharacterTab::Quests => render_quests_tab(commands, content, quests, selected),
        CharacterTab::Journal => {
            render_journal_tab(commands, content, character, history, selected);
        }
    }

}

fn render_character_general_tab(
    commands: &mut Commands,
    parent: Entity,
    view: &CharacterSheetView,
) {
    bevy_presentation::spawn_label(
        commands,
        parent,
        format!(
            "Health: {}/{} | Experience: {}/{}",
            view.character.hp, view.character.max_hp, view.experience, view.next_level_experience
        ),
    );
    bevy_presentation::spawn_label(
        commands,
        parent,
        format!(
            "Might {} (effective {}) | Insight {} (effective {}) | Endurance {} (effective {})",
            view.attributes.might,
            view.attributes.effective_might,
            view.attributes.insight,
            view.attributes.effective_insight,
            view.attributes.endurance,
            view.attributes.effective_endurance,
        ),
    );
    if view.conditions.is_empty() {
        bevy_presentation::spawn_muted_label(commands, parent, "Conditions: none");
    } else {
        bevy_presentation::spawn_label(commands, parent, "Conditions");
        for condition in &view.conditions {
            let effect = match (condition.penalty, condition.bonus) {
                (penalty, bonus) if penalty < 0 && bonus > 0 => {
                    format!(" {:+} penalty, {:+} bonus", penalty, bonus)
                }
                (penalty, _) if penalty < 0 => format!(" {:+} penalty", penalty),
                (_, bonus) if bonus > 0 => format!(" {:+} bonus", bonus),
                _ => String::new(),
            };
            bevy_presentation::spawn_muted_label(
                commands,
                parent,
                format!("{} | {} portions{}", condition.name, condition.remaining, effect),
            );
        }
    }
}

fn render_character_reputation_tab(
    commands: &mut Commands,
    parent: Entity,
    view: &CharacterSheetView,
) {
    if view.factions.is_empty() {
        bevy_presentation::spawn_muted_label(
            commands,
            parent,
            "No faction reputations have been recorded.",
        );
    } else {
        for faction in &view.factions {
            bevy_presentation::spawn_label(
                commands,
                parent,
                format!("{} {:+}", faction.name, faction.reputation),
            );
            for memory in faction.memories.iter().rev() {
                bevy_presentation::spawn_muted_label(commands, parent, format!("  | {memory}"));
            }
        }
    }
}

fn render_inventory_tab(
    commands: &mut Commands,
    parent: Entity,
    view: &InventoryView,
    selected: usize,
) {
    if view.items.is_empty() {
        bevy_presentation::spawn_muted_label(commands, parent, "Your pack is empty.");
        return;
    }
    for (index, item) in view.items.iter().enumerate() {
        let button = bevy_presentation::spawn_action_button(
            commands,
            parent,
            index,
            "item",
            item.name.clone(),
        );
        if selected == index {
            commands.entity(button).insert(bevy_presentation::UiSelected);
        }
    }
}

fn render_quest_detail(commands: &mut Commands, view: Option<&QuestView>) {
    let panel = render_header(
        commands,
        "QUEST | DETAILS",
        "Objective progress and reward.",
    );
    let Some(view) = view else {
        bevy_presentation::spawn_muted_label(commands, panel, "That quest is no longer available.");
        let button = bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back");
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
        return;
    };
    bevy_presentation::spawn_label(commands, panel, &view.title);
    bevy_presentation::spawn_muted_label(commands, panel, format!("Status: {}", view.status));
    if !view.description.trim().is_empty() {
        bevy_presentation::spawn_muted_label(commands, panel, &view.description);
    }
    bevy_presentation::spawn_label(commands, panel, "Objectives");
    if view.objectives.is_empty() {
        bevy_presentation::spawn_muted_label(commands, panel, "No objectives recorded.");
    } else {
        for objective in &view.objectives {
            let marker = if objective.completed { "x" } else { " " };
            bevy_presentation::spawn_muted_label(
                commands,
                panel,
                format!(
                    "[{marker}] {} ({}/{})",
                    objective.label, objective.progress, objective.required
                ),
            );
        }
    }
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        format!(
            "Reward: {}",
            view.reward_item_name.as_deref().unwrap_or("-")
        ),
    );
    let button =
        bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back to quest log");
    commands
        .entity(button)
        .insert(bevy_presentation::UiSelected);
}

fn render_meditation(
    commands: &mut Commands,
    view: &crate::presentation::MeditationView,
    selected: usize,
    message: Option<&str>,
) {
    let panel = render_header(commands, "MEDITATION", &view.current_time);
    if let Some(message) = message.or(view.unavailable_message.as_deref()) {
        bevy_presentation::spawn_muted_label(commands, panel, message);
    }
    if !view.safe_to_meditate {
        let button = bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back");
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
        return;
    }
    bevy_presentation::spawn_muted_label(commands, panel, "Choose when to end your meditation.");
    for (index, target) in view.targets.iter().enumerate() {
        let button = bevy_presentation::spawn_action_button(
            commands,
            panel,
            index,
            "meditate",
            target.label.clone(),
        );
        if selected == index {
            commands
                .entity(button)
                .insert(bevy_presentation::UiSelected);
        }
    }
    let back = view.targets.len();
    let button = bevy_presentation::spawn_action_button(commands, panel, back, "cancel", "Cancel");
    if selected == back {
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
    }
}

fn render_meditation_result(
    commands: &mut Commands,
    result: Option<&crate::presentation::MeditationResultView>,
) {
    let panel = render_header(
        commands,
        "MEDITATION | COMPLETE",
        "The result of your rest.",
    );
    let Some(result) = result else {
        bevy_presentation::spawn_muted_label(commands, panel, "No meditation result is available.");
        let button = bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back");
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
        return;
    };
    bevy_presentation::spawn_label(commands, panel, &result.ending_time);
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        format!("Time meditated: {} portion(s)", result.portions),
    );
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        format!("HP recovered: {}", result.hp_recovered),
    );
    if result.exhausted_removed {
        bevy_presentation::spawn_muted_label(commands, panel, "Exhausted is removed.");
    }
    if result.well_rested_applied {
        bevy_presentation::spawn_muted_label(commands, panel, "Well-rested is applied.");
    }
    let button =
        bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back to gameplay");
    commands
        .entity(button)
        .insert(bevy_presentation::UiSelected);
}

fn render_chronicle_detail(commands: &mut Commands, entry: Option<&HistoryEntryView>) {
    let panel = render_header(commands, "JOURNAL | CHRONICLE", "Recorded world events.");
    let Some(entry) = entry else {
        bevy_presentation::spawn_muted_label(
            commands,
            panel,
            "That history entry is no longer available.",
        );
        let button = bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back");
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
        return;
    };
    bevy_presentation::spawn_label(commands, panel, format!("Day {}", entry.day));
    bevy_presentation::spawn_muted_label(commands, panel, &entry.text);
    if let Some(event_id) = &entry.event_id {
        bevy_presentation::spawn_muted_label(commands, panel, format!("Event: {event_id}"));
    }
    if let Some(location) = &entry.location_name {
        bevy_presentation::spawn_muted_label(commands, panel, format!("Location: {location}"));
    }
    if let Some(outcome) = &entry.outcome {
        bevy_presentation::spawn_muted_label(commands, panel, format!("Outcome: {outcome}"));
    }
    let button =
        bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back to journal");
    commands
        .entity(button)
        .insert(bevy_presentation::UiSelected);
}

fn render_journal_entry(
    commands: &mut Commands,
    state: &crate::model::GameState,
    draft: &str,
    selected: usize,
    message: Option<&str>,
) {
    let panel = render_header(
        commands,
        "WRITE IN JOURNAL",
        state.character.display_name(),
    );
    if let Some(message) = message {
        bevy_presentation::spawn_label(commands, panel, message);
        let button =
            bevy_presentation::spawn_action_button(commands, panel, 0, "continue", "Continue");
        commands.entity(button).insert(bevy_presentation::UiSelected);
        return;
    }
    bevy_presentation::spawn_muted_label(commands, panel, "Type a note. Backspace edits it.");
    bevy_presentation::spawn_label(
        commands,
        panel,
        if draft.is_empty() {
            "_".to_string()
        } else {
            draft.to_string()
        },
    );
    for (index, (icon, label)) in [("write", "Record note"), ("cancel", "Cancel")]
        .into_iter()
        .enumerate()
    {
        let button = bevy_presentation::spawn_action_button(commands, panel, index, icon, label);
        if selected == index {
            commands.entity(button).insert(bevy_presentation::UiSelected);
        }
    }
}

fn entry_marker(entry: &HistoryEntryView) -> &'static str {
    match entry.entry_type {
        crate::presentation::HistoryEntryViewType::Event => "[EVENT]",
        crate::presentation::HistoryEntryViewType::Narrative => "[NOTE]",
    }
}

#[cfg(test)]
#[path = "../tests/unit/bevy_records.rs"]
mod tests;
