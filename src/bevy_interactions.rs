//! Bevy presentation and interaction for NPC dialogue and character remains.
//!
//! The interaction rules remain in the game modules. This adapter only owns
//! screen state, selection, and rendering for the graphical frontend.

use bevy::prelude::*;

use crate::bevy_lifecycle::{LifecyclePhase, LifecycleState};
use crate::bevy_presentation::{
    self, BevyScreenRoot, GameplayInputQueue, NavigationState, ScreenId,
};
use crate::game::{interactions, legacy};
use crate::input::InputEvent;
use crate::model::EntityId;
use crate::presentation::{ConversationView, RemainsResultView, RemainsView, TalkView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteractionScreen {
    Talk,
    Conversation,
    Remains,
    RemainsResult,
}

#[derive(Resource)]
pub(crate) struct BevyInteractionState {
    screen: InteractionScreen,
    selected: usize,
    npc_id: Option<EntityId>,
    result: Option<RemainsResultView>,
    message: Option<String>,
    dirty: bool,
}

impl Default for BevyInteractionState {
    fn default() -> Self {
        Self {
            screen: InteractionScreen::Talk,
            selected: 0,
            npc_id: None,
            result: None,
            message: None,
            dirty: true,
        }
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<BevyInteractionState>().add_systems(
        Update,
        (sync_screen, interaction_input, render_if_active).chain(),
    );
}

fn is_interaction_screen(screen: Option<ScreenId>) -> bool {
    matches!(
        screen,
        Some(ScreenId::Talk | ScreenId::Conversation | ScreenId::Remains | ScreenId::RemainsResult)
    )
}

fn sync_screen(
    lifecycle: Res<LifecycleState>,
    navigation: Res<NavigationState>,
    mut state: ResMut<BevyInteractionState>,
) {
    if lifecycle.phase != LifecyclePhase::Complete
        || !is_interaction_screen(navigation.current_screen)
    {
        return;
    }
    let Some(screen) = navigation.current_screen else {
        return;
    };

    let target = match screen {
        ScreenId::Talk => InteractionScreen::Talk,
        ScreenId::Conversation => InteractionScreen::Conversation,
        ScreenId::Remains => InteractionScreen::Remains,
        ScreenId::RemainsResult => InteractionScreen::RemainsResult,
        _ => return,
    };
    if state.screen == target && state.dirty {
        return;
    }
    if state.screen != target {
        state.screen = target;
        state.selected = navigation.selected;
        state.message = None;
        state.dirty = true;
    }
}

fn selection_count(state: &BevyInteractionState, lifecycle: &LifecycleState) -> usize {
    let Some(session) = lifecycle.session.as_ref() else {
        return 1;
    };
    match state.screen {
        InteractionScreen::Talk => {
            interactions::build_talk_view_for_bevy(&session.state)
                .npcs
                .len()
                + 1
        }
        InteractionScreen::Conversation => state
            .npc_id
            .and_then(|npc_id| interactions::npc_index_by_id(&session.state, npc_id))
            .and_then(|npc_index| {
                interactions::build_conversation_view_for_bevy(&session.state, npc_index)
                    .map(|view| view.options.len() + 1)
            })
            .unwrap_or(1),
        InteractionScreen::Remains => {
            legacy::build_remains_view_for_bevy(&session.state)
                .remains
                .len()
                + 1
        }
        InteractionScreen::RemainsResult => 1,
    }
}

fn move_selection(
    state: &mut BevyInteractionState,
    lifecycle: &LifecycleState,
    navigation: &mut NavigationState,
    direction: isize,
) {
    let count = selection_count(state, lifecycle);
    if count == 0 {
        return;
    }
    state.selected = (state.selected as isize + direction).rem_euclid(count as isize) as usize;
    navigation.selected = state.selected;
    state.dirty = true;
}

fn move_to_end(
    state: &mut BevyInteractionState,
    lifecycle: &LifecycleState,
    navigation: &mut NavigationState,
) {
    let count = selection_count(state, lifecycle);
    if count > 0 {
        state.selected = count - 1;
        navigation.selected = state.selected;
        state.dirty = true;
    }
}

fn close_to_gameplay(state: &mut BevyInteractionState, navigation: &mut NavigationState) {
    state.selected = 0;
    state.npc_id = None;
    state.result = None;
    state.message = None;
    state.dirty = true;
    navigation.current_screen = Some(ScreenId::Gameplay);
    navigation.return_screen = None;
    navigation.selected = 0;
}

fn interaction_input(
    mut lifecycle: ResMut<LifecycleState>,
    mut state: ResMut<BevyInteractionState>,
    mut navigation: ResMut<NavigationState>,
    mut input_queue: ResMut<GameplayInputQueue>,
) {
    if lifecycle.phase != LifecyclePhase::Complete
        || !is_interaction_screen(navigation.current_screen)
        || input_queue.0.is_empty()
    {
        return;
    }

    let events = std::mem::take(&mut input_queue.0);
    for event in events {
        match event {
            InputEvent::Up => move_selection(&mut state, &lifecycle, &mut navigation, -1),
            InputEvent::Down => move_selection(&mut state, &lifecycle, &mut navigation, 1),
            InputEvent::Home => {
                state.selected = 0;
                navigation.selected = 0;
                state.dirty = true;
            }
            InputEvent::End => move_to_end(&mut state, &lifecycle, &mut navigation),
            InputEvent::PageUp => move_selection(&mut state, &lifecycle, &mut navigation, -5),
            InputEvent::PageDown => move_selection(&mut state, &lifecycle, &mut navigation, 5),
            InputEvent::Confirm => activate(&mut lifecycle, &mut state, &mut navigation),
            InputEvent::Cancel => cancel(&mut state, &mut navigation),
            _ => {}
        }
    }
}

fn activate(
    lifecycle: &mut LifecycleState,
    state: &mut BevyInteractionState,
    navigation: &mut NavigationState,
) {
    let Some(session) = lifecycle.session.as_mut() else {
        return;
    };

    match state.screen {
        InteractionScreen::Talk => {
            let view = interactions::build_talk_view_for_bevy(&session.state);
            if state.selected >= view.npcs.len() {
                close_to_gameplay(state, navigation);
                return;
            }
            let Some(npc_id) = interactions::npc_ids_at_location(
                &session.state,
                session.state.character.location_id,
            )
            .get(state.selected)
            .copied() else {
                state.message = Some("There is no one here to talk to.".to_string());
                state.dirty = true;
                return;
            };
            state.npc_id = Some(npc_id);
            state.screen = InteractionScreen::Conversation;
            state.selected = 0;
            state.message = None;
            navigation.current_screen = Some(ScreenId::Conversation);
            navigation.selected = 0;
            state.dirty = true;
        }
        InteractionScreen::Conversation => {
            if state.message.is_some() {
                state.message = None;
                state.selected = 0;
                navigation.selected = 0;
                state.dirty = true;
                return;
            }
            let Some(npc_id) = state.npc_id else {
                state.screen = InteractionScreen::Talk;
                navigation.current_screen = Some(ScreenId::Talk);
                state.selected = 0;
                state.dirty = true;
                return;
            };
            let option_count = interactions::npc_index_by_id(&session.state, npc_id)
                .and_then(|npc_index| {
                    interactions::build_conversation_view_for_bevy(&session.state, npc_index)
                        .map(|view| view.options.len())
                })
                .unwrap_or(0);
            if state.selected >= option_count {
                state.screen = InteractionScreen::Talk;
                state.selected = 0;
                navigation.current_screen = Some(ScreenId::Talk);
                navigation.selected = 0;
                state.dirty = true;
                return;
            }
            let messages = interactions::perform_conversation_choice(
                &mut session.state,
                npc_id,
                state.selected,
            );
            state.message = Some(messages.join("\n"));
            state.selected = 0;
            navigation.selected = 0;
            state.dirty = true;
        }
        InteractionScreen::Remains => {
            let view = legacy::build_remains_view_for_bevy(&session.state);
            if state.selected >= view.remains.len() {
                close_to_gameplay(state, navigation);
                return;
            }
            match legacy::search_remains_for_bevy(&mut session.state, state.selected) {
                Ok(result) => {
                    state.result = Some(result);
                    state.screen = InteractionScreen::RemainsResult;
                    state.selected = 0;
                    navigation.current_screen = Some(ScreenId::RemainsResult);
                    navigation.selected = 0;
                    state.dirty = true;
                }
                Err(error) => {
                    state.message = Some(error);
                    state.dirty = true;
                }
            }
        }
        InteractionScreen::RemainsResult => close_to_gameplay(state, navigation),
    }
}

fn cancel(state: &mut BevyInteractionState, navigation: &mut NavigationState) {
    match state.screen {
        InteractionScreen::Talk | InteractionScreen::Remains => {
            close_to_gameplay(state, navigation)
        }
        InteractionScreen::Conversation => {
            state.screen = InteractionScreen::Talk;
            state.selected = 0;
            state.message = None;
            navigation.current_screen = Some(ScreenId::Talk);
            navigation.selected = 0;
            state.dirty = true;
        }
        InteractionScreen::RemainsResult => close_to_gameplay(state, navigation),
    }
}

fn render_if_active(
    mut commands: Commands,
    lifecycle: Res<LifecycleState>,
    navigation: Res<NavigationState>,
    mut state: ResMut<BevyInteractionState>,
    roots: Query<Entity, With<BevyScreenRoot>>,
) {
    if lifecycle.phase != LifecyclePhase::Complete
        || !is_interaction_screen(navigation.current_screen)
        || !state.dirty
    {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some(session) = lifecycle.session.as_ref() else {
        return;
    };

    match state.screen {
        InteractionScreen::Talk => {
            let view = interactions::build_talk_view_for_bevy(&session.state);
            render_talk(&mut commands, &view, state.selected);
        }
        InteractionScreen::Conversation => {
            let view = state
                .npc_id
                .and_then(|npc_id| interactions::npc_index_by_id(&session.state, npc_id))
                .and_then(|npc_index| {
                    interactions::build_conversation_view_for_bevy(&session.state, npc_index)
                });
            render_conversation(
                &mut commands,
                view.as_ref(),
                state.selected,
                state.message.as_deref(),
            );
        }
        InteractionScreen::Remains => {
            let view = legacy::build_remains_view_for_bevy(&session.state);
            render_remains(
                &mut commands,
                &view,
                state.selected,
                state.message.as_deref(),
            );
        }
        InteractionScreen::RemainsResult => {
            render_remains_result(&mut commands, state.result.as_ref())
        }
    }
    state.dirty = false;
}

fn render_talk(commands: &mut Commands, view: &TalkView, selected: usize) {
    let root = bevy_presentation::spawn_screen(commands, "TALK");
    let panel = bevy_presentation::spawn_panel(commands, root);
    bevy_presentation::spawn_muted_label(commands, panel, "Choose someone to speak with.");
    if view.npcs.is_empty() {
        bevy_presentation::spawn_muted_label(commands, panel, "There is no one here to talk to.");
    } else {
        for (index, npc) in view.npcs.iter().enumerate() {
            let button = bevy_presentation::spawn_action_button(
                commands,
                panel,
                index,
                "character",
                npc.display_name(),
            );
            if index == selected {
                commands
                    .entity(button)
                    .insert(bevy_presentation::UiSelected);
            }
        }
    }
    let back_index = view.npcs.len();
    let button =
        bevy_presentation::spawn_action_button(commands, panel, back_index, "back", "Back");
    if selected == back_index {
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
    }
}

fn render_conversation(
    commands: &mut Commands,
    view: Option<&ConversationView>,
    selected: usize,
    message: Option<&str>,
) {
    let root = bevy_presentation::spawn_screen(commands, "CONVERSATION");
    let panel = bevy_presentation::spawn_panel(commands, root);
    let Some(view) = view else {
        bevy_presentation::spawn_muted_label(
            commands,
            panel,
            "That person can no longer be found.",
        );
        let button = bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back");
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
        return;
    };
    bevy_presentation::spawn_label(commands, panel, view.npc.display_name());
    if let Some(faction) = &view.npc.faction_name {
        bevy_presentation::spawn_muted_label(commands, panel, format!("Faction: {faction}"));
    }
    if let Some(portrait) = &view.portrait {
        bevy_presentation::spawn_muted_label(commands, panel, portrait.clone());
    }
    if let Some(memory) = &view.memory {
        bevy_presentation::spawn_muted_label(commands, panel, format!("Memory: {memory}"));
    }
    if !view.available {
        bevy_presentation::spawn_muted_label(
            commands,
            panel,
            view.unavailable_message
                .as_deref()
                .unwrap_or("They are unavailable right now."),
        );
    } else if let Some(message) = message {
        for line in message.lines() {
            bevy_presentation::spawn_muted_label(commands, panel, line);
        }
        let button =
            bevy_presentation::spawn_action_button(commands, panel, 0, "continue", "Continue");
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
    } else {
        for (index, option) in view.options.iter().enumerate() {
            let button =
                bevy_presentation::spawn_action_button(commands, panel, index, "choice", option);
            if index == selected {
                commands
                    .entity(button)
                    .insert(bevy_presentation::UiSelected);
            }
        }
        let back_index = view.options.len();
        let button =
            bevy_presentation::spawn_action_button(commands, panel, back_index, "back", "Back");
        if selected == back_index {
            commands
                .entity(button)
                .insert(bevy_presentation::UiSelected);
        }
    }
}

fn render_remains(
    commands: &mut Commands,
    view: &RemainsView,
    selected: usize,
    message: Option<&str>,
) {
    let root = bevy_presentation::spawn_screen(commands, "REMAINS");
    let panel = bevy_presentation::spawn_panel(commands, root);
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        format!("Search the remains at {}.", view.location_name),
    );
    if let Some(message) = message {
        bevy_presentation::spawn_muted_label(commands, panel, message);
    }
    if view.remains.is_empty() {
        bevy_presentation::spawn_muted_label(
            commands,
            panel,
            "There are no remains worth searching here.",
        );
    } else {
        for (index, remains) in view.remains.iter().enumerate() {
            let button = bevy_presentation::spawn_action_button(
                commands,
                panel,
                index,
                "remains",
                remains.label.clone(),
            );
            if index == selected {
                commands
                    .entity(button)
                    .insert(bevy_presentation::UiSelected);
            }
        }
    }
    let back_index = view.remains.len();
    let button =
        bevy_presentation::spawn_action_button(commands, panel, back_index, "back", "Back");
    if selected == back_index {
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
    }
}

fn render_remains_result(commands: &mut Commands, result: Option<&RemainsResultView>) {
    let root = bevy_presentation::spawn_screen(commands, "REMAINS RECOVERED");
    let panel = bevy_presentation::spawn_panel(commands, root);
    let Some(result) = result else {
        bevy_presentation::spawn_muted_label(commands, panel, "Nothing was recovered.");
        let button = bevy_presentation::spawn_action_button(commands, panel, 0, "back", "Back");
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
        return;
    };
    bevy_presentation::spawn_label(
        commands,
        panel,
        format!(
            "{} the {} at {}",
            result.former_name, result.former_title, result.location_name
        ),
    );
    for item in &result.items {
        bevy_presentation::spawn_label(commands, panel, format!("Recovered: {}", item.name));
        if !item.description.is_empty() {
            bevy_presentation::spawn_muted_label(commands, panel, item.description.clone());
        }
    }
    if let Some(item) = &result.hidden_item {
        bevy_presentation::spawn_label(commands, panel, format!("Discovered: {}", item.name));
        bevy_presentation::spawn_muted_label(commands, panel, item.description.clone());
    }
    for note in &result.notes {
        bevy_presentation::spawn_muted_label(commands, panel, note.clone());
    }
    let button = bevy_presentation::spawn_action_button(commands, panel, 0, "continue", "Continue");
    commands
        .entity(button)
        .insert(bevy_presentation::UiSelected);
}

#[cfg(test)]
#[path = "../tests/unit/bevy_interactions.rs"]
mod tests;
