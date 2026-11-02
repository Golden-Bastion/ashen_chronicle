//! Bevy lifecycle flow: start, load, character creation, quit, and death.
//!
//! This module owns only frontend orchestration. Game creation, loading,
//! validation, inheritance, and save-path rules remain in the existing model,
//! persistence, and game modules.

use crate::bevy_presentation::{
    self, BevyScreenRoot, LifecycleTextField, NavigationState, ScreenId, SemanticInputQueue,
};
use crate::game::validate_loaded_state;
use crate::game_paths::GamePaths;
use crate::input::InputEvent;
use crate::model::{create_inherited_state, create_new_state, GameState, WorldMode};
use crate::persistence::{
    character_save_path, find_save_files, legacy_save_path, load_game_with_metadata,
};
use crate::presentation::{DeathView, FactionView, ItemView, ScreenView};
use bevy::input_focus::{FocusCause, FocusGained, InputFocus};
use bevy::prelude::*;
use bevy::text::EditableText;
#[cfg(any(target_os = "android", test))]
use bevy::text::TextEdit;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "android")]
#[derive(Resource, Default)]
struct AndroidLifecycleTextSync {
    focused: Option<Entity>,
    last_sent_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LifecyclePhase {
    Start,
    Load,
    CreateCharacter,
    QuitConfirm,
    Death,
    Complete,
}

#[derive(Resource)]
pub(crate) struct LifecycleState {
    pub(crate) phase: LifecyclePhase,
    pub(crate) save_files: Vec<PathBuf>,
    pub(crate) selected: usize,
    pub(crate) world_name: String,
    pub(crate) character_name: String,
    pub(crate) character_title: String,
    pub(crate) pending_state: Option<(GameState, PathBuf)>,
    pub(crate) session: Option<GameSession>,
    pub(crate) message: Option<String>,
    quit_variant: usize,
    dirty: bool,
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self {
            phase: LifecyclePhase::Start,
            save_files: Vec::new(),
            selected: 0,
            world_name: "The Ashen Crown".to_string(),
            character_name: String::new(),
            character_title: "Ash Walker".to_string(),
            pending_state: None,
            session: None,
            message: None,
            quit_variant: 0,
            dirty: true,
        }
    }
}

#[derive(Resource)]
pub(crate) struct GameSession {
    pub(crate) state: GameState,
    pub(crate) save_path: PathBuf,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<LifecycleState>()
        .add_systems(Startup, initialize)
        .add_systems(
            Update,
            (
                lifecycle_input,
                sync_character_field_values,
                render_if_dirty,
            )
                .chain(),
        )
        .add_observer(on_lifecycle_field_focus_gained);

    #[cfg(target_os = "android")]
    app.init_resource::<AndroidLifecycleTextSync>()
        .add_systems(Update, sync_android_lifecycle_text_input);
}

fn initialize(mut lifecycle: ResMut<LifecycleState>) {
    lifecycle.refresh_saves();
}

fn lifecycle_input(
    mut lifecycle: ResMut<LifecycleState>,
    mut navigation: ResMut<NavigationState>,
    mut input_queue: ResMut<SemanticInputQueue>,
    mut input_focus: ResMut<InputFocus>,
    fields: Query<(Entity, &LifecycleTextField)>,
    #[cfg(not(target_os = "android"))] mut commands: Commands,
) {
    if input_queue.0.is_empty() {
        return;
    }

    let events = std::mem::take(&mut input_queue.0);
    for event in events {
        match event {
            InputEvent::Up => move_selection(&mut lifecycle, &mut navigation, -1),
            InputEvent::Down => move_selection(&mut lifecycle, &mut navigation, 1),
            InputEvent::Home => set_selection(&mut lifecycle, &mut navigation, 0),
            InputEvent::End => {
                let max = max_selection(&lifecycle);
                set_selection(&mut lifecycle, &mut navigation, max);
            }
            InputEvent::Cancel => handle_cancel(&mut lifecycle, &mut navigation),
            InputEvent::Confirm => {
                if let Some(exit) = activate_selection(&mut lifecycle, &mut navigation) {
                    if exit {
                        #[cfg(target_os = "android")]
                        {
                            request_android_game_exit();
                        }
                        #[cfg(not(target_os = "android"))]
                        {
                            commands.write_message(AppExit::Success);
                        }
                        return;
                    }
                }
            }
            _ => {}
        }
    }

    if lifecycle.phase == LifecyclePhase::CreateCharacter && lifecycle.selected <= 2 {
        if let Some((entity, _)) = fields
            .iter()
            .find(|(_, field)| field.index == lifecycle.selected)
        {
            if input_focus.get() != Some(entity) {
                input_focus.set(entity, FocusCause::Navigated);
            }
        }
    } else if input_focus
        .get()
        .is_some_and(|entity| fields.get(entity).is_ok())
    {
        input_focus.clear();
    }
}

#[cfg(any(target_os = "android", test))]
fn replace_text_snapshot(field: &mut EditableText, text: &str) {
    // Native Android input sends the entire current value, not an insertion delta.
    // Replace the buffer directly so Parley never inserts a full snapshot at a stale cursor.
    // Bevy 0.20's clear() also queues a cursor-to-start edit; direct replacement
    // makes that edit obsolete, so replace it with the required cursor-to-end edit.
    field.clear();
    field.editor.set_text(text);
    field.pending_edits.clear();
    field.queue_edit(TextEdit::TextEnd(false));
}

#[cfg(target_os = "android")]
fn sync_android_lifecycle_text_input(
    lifecycle: Res<LifecycleState>,
    mut input_focus: ResMut<InputFocus>,
    mut sync: ResMut<AndroidLifecycleTextSync>,
    mut input_queue: ResMut<SemanticInputQueue>,
    mut fields: Query<(Entity, &mut EditableText), With<LifecycleTextField>>,
) {
    let focused = if lifecycle.phase == LifecyclePhase::CreateCharacter {
        input_focus
            .get()
            .filter(|entity| fields.get(*entity).is_ok())
    } else {
        None
    };

    if sync.focused != focused {
        sync.focused = focused;
        if let Some(entity) = focused {
            if let Ok((_, field)) = fields.get(entity) {
                sync.last_sent_text = field.value().to_string();
                bevy_presentation::android_set_text_input_target(Some(0), &sync.last_sent_text);
            }
        } else {
            sync.last_sent_text.clear();
            bevy_presentation::android_set_text_input_target(None, "");
        }
        return;
    }

    let Some(entity) = focused else {
        return;
    };
    let Ok((_, mut field)) = fields.get_mut(entity) else {
        return;
    };

    for event in bevy_presentation::android_take_text_input_events(0) {
        match event {
            bevy_presentation::AndroidTextInputEvent::Changed(text) => {
                let text = text
                    .chars()
                    .filter(|character| !character.is_control())
                    .collect::<String>();

                if field.value() != &text {
                    replace_text_snapshot(&mut field, &text);
                }
                sync.last_sent_text = text;
            }
            bevy_presentation::AndroidTextInputEvent::Submitted => {
                input_queue.0.push(InputEvent::Confirm);
            }
            bevy_presentation::AndroidTextInputEvent::Dismissed => {
                input_focus.clear();
                sync.focused = None;
                sync.last_sent_text.clear();
            }
        }
    }
}

fn sync_character_field_values(
    mut lifecycle: ResMut<LifecycleState>,
    fields: Query<(&EditableText, &LifecycleTextField), Changed<EditableText>>,
) {
    if lifecycle.phase != LifecyclePhase::CreateCharacter {
        return;
    }

    for (field, marker) in &fields {
        let value = field.value().to_string();
        match marker.index {
            0 if lifecycle.world_name != value => lifecycle.world_name = value,
            1 if lifecycle.character_name != value => lifecycle.character_name = value,
            2 if lifecycle.character_title != value => lifecycle.character_title = value,
            _ => {}
        }
    }
}

fn on_lifecycle_field_focus_gained(
    trigger: On<FocusGained>,
    fields: Query<&LifecycleTextField>,
    mut lifecycle: ResMut<LifecycleState>,
    mut navigation: ResMut<NavigationState>,
) {
    if lifecycle.phase != LifecyclePhase::CreateCharacter {
        return;
    }

    let Ok(field) = fields.get(trigger.event_target()) else {
        return;
    };

    lifecycle.selected = field.index;
    navigation.current_screen = Some(ScreenId::Lifecycle);
    navigation.selected = field.index;
}

fn max_selection(lifecycle: &LifecycleState) -> usize {
    match lifecycle.phase {
        LifecyclePhase::Start => start_option_count(lifecycle).saturating_sub(1),
        LifecyclePhase::Load => lifecycle.save_files.len(),
        LifecyclePhase::CreateCharacter => 3,
        LifecyclePhase::QuitConfirm => 1,
        LifecyclePhase::Death => 2,
        LifecyclePhase::Complete => 0,
    }
}

fn set_selection(
    lifecycle: &mut LifecycleState,
    navigation: &mut NavigationState,
    selected: usize,
) {
    lifecycle.selected = selected.min(max_selection(lifecycle));
    navigation.selected = lifecycle.selected;
    if lifecycle.phase != LifecyclePhase::CreateCharacter {
        lifecycle.dirty = true;
    }
}

fn move_selection(
    lifecycle: &mut LifecycleState,
    navigation: &mut NavigationState,
    direction: isize,
) {
    let max = max_selection(lifecycle);
    let selected = (lifecycle.selected as isize + direction).clamp(0, max as isize) as usize;
    set_selection(lifecycle, navigation, selected);
}

fn activate_selection(
    lifecycle: &mut LifecycleState,
    navigation: &mut NavigationState,
) -> Option<bool> {
    match lifecycle.phase {
        LifecyclePhase::Start => {
            if lifecycle.selected == 0 {
                lifecycle.start_new_game(navigation);
            } else if start_has_load(lifecycle) && lifecycle.selected == 1 {
                lifecycle.start_load_game(navigation);
            } else if lifecycle.selected == start_options_index(lifecycle) {
                navigation.return_screen = Some(ScreenId::Lifecycle);
                navigation.current_screen = Some(ScreenId::Options);
                navigation.selected = 0;
                return None;
            } else {
                return Some(true);
            }
        }
        LifecyclePhase::Load => {
            if lifecycle.selected == lifecycle.save_files.len() {
                lifecycle.phase = LifecyclePhase::Start;
                lifecycle.selected = 0;
                lifecycle.message = None;
                lifecycle.pending_state = None;
                navigation.return_screen = None;
                navigation.current_screen = Some(ScreenId::Lifecycle);
                navigation.selected = 0;
                lifecycle.dirty = true;
                return None;
            }
            if let Some(path) = lifecycle.save_files.get(lifecycle.selected).cloned() {
                match load_game_with_metadata(&path) {
                    Ok(loaded) => {
                        let state = loaded.game;
                        let mut warnings = validate_loaded_state(&state);
                        let active_mods = crate::mod_manager::active_mods();
                        for dependency in &loaded.mod_dependencies {
                            match active_mods
                                .iter()
                                .find(|item| item.manifest.id == dependency.id)
                            {
                                None => warnings.push(format!(
                                    "Missing mod: {} ({}) version {}.",
                                    dependency.name, dependency.id, dependency.version
                                )),
                                Some(item) if !item.enabled => warnings.push(format!(
                                    "Required mod is disabled: {} ({}) version {}.",
                                    dependency.name, dependency.id, dependency.version
                                )),
                                Some(item) if item.manifest.version != dependency.version => {
                                    warnings.push(format!(
                                        "Mod version mismatch: {} ({}) save={} installed={}.",
                                        dependency.name,
                                        dependency.id,
                                        dependency.version,
                                        item.manifest.version
                                    ))
                                }
                                Some(_) => {}
                            }
                        }
                        // Keep the selected slot as the active save path, including
                        // imported slots with collision-safe alternate filenames.
                        lifecycle.pending_state = Some((state, path.clone()));
                        lifecycle.message = if warnings.is_empty() {
                            None
                        } else {
                            Some(format!(
                                "Save has warnings. Confirm to continue at your own responsibility:\n{}",
                                warnings.join("\n")
                            ))
                        };
                        if warnings.is_empty() {
                            lifecycle.finish_loading();
                        } else {
                            lifecycle.phase = LifecyclePhase::Complete;
                            lifecycle.selected = 0;
                        }
                        lifecycle.dirty = true;
                    }
                    Err(err) => {
                        lifecycle.message = Some(format!("Could not load save: {err}"));
                        lifecycle.dirty = true;
                    }
                }
            }
        }
        LifecyclePhase::CreateCharacter => match lifecycle.selected {
            0..=2 => lifecycle.selected = (lifecycle.selected + 1).min(3),
            3 => lifecycle.create_character(),
            _ => {}
        },
        LifecyclePhase::QuitConfirm => {
            if lifecycle.selected == 0 {
                return Some(true);
            }
            lifecycle.cancel_quit_confirmation(navigation);
        }
        LifecyclePhase::Death => match lifecycle.selected {
            0 => {
                lifecycle.create_character();
                lifecycle.message =
                    Some("A new world begins. The old world remains behind.".to_string());
            }
            1 => lifecycle.inherit_character(),
            2 => return Some(true),
            _ => {}
        },
        LifecyclePhase::Complete => {
            if lifecycle.pending_state.is_some() {
                lifecycle.finish_loading();
            } else {
                return Some(true);
            }
        }
    }
    navigation.current_screen = Some(ScreenId::Lifecycle);
    navigation.selected = lifecycle.selected;
    None
}

#[cfg(target_os = "android")]
fn request_android_game_exit() {
    let Some(app) = bevy::android::ANDROID_APP.get().cloned() else {
        bevy::log::warn!("Could not request Android game exit: Android app handle is unavailable");
        return;
    };
    let java_app = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(java_app.vm_as_ptr().cast()) };
        if let Err(error) = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let raw_activity = java_app.activity_as_ptr() as jni::sys::jobject;
            let activity = unsafe {
                env.as_cast_raw::<jni::refs::Global<jni::objects::JObject>>(&raw_activity)?
            };
            env.call_method(
                activity.as_ref(),
                jni::jni_str!("requestGameExit"),
                jni::jni_sig!("()V"),
                &[],
            )?;
            Ok(())
        }) {
            bevy::log::warn!("Could not request Android game exit: {error}");
        }
    }));
}

fn handle_cancel(lifecycle: &mut LifecycleState, navigation: &mut NavigationState) {
    match lifecycle.phase {
        LifecyclePhase::Start => lifecycle.start_quit_confirmation(navigation, None),
        LifecyclePhase::Load | LifecyclePhase::CreateCharacter => {
            lifecycle.phase = LifecyclePhase::Start;
            lifecycle.selected = 0;
            lifecycle.message = None;
            navigation.return_screen = None;
            navigation.current_screen = Some(ScreenId::Lifecycle);
            navigation.selected = 0;
            lifecycle.dirty = true;
        }
        LifecyclePhase::QuitConfirm => lifecycle.cancel_quit_confirmation(navigation),
        LifecyclePhase::Death => {
            lifecycle.phase = LifecyclePhase::Start;
            lifecycle.selected = 0;
            lifecycle.message = None;
            navigation.return_screen = None;
            navigation.current_screen = Some(ScreenId::Lifecycle);
            navigation.selected = 0;
            lifecycle.dirty = true;
        }
        LifecyclePhase::Complete => {}
    }
}

fn available_save_files() -> Vec<PathBuf> {
    let paths = GamePaths::get();
    let mut save_files = find_save_files(&paths.saves_dir).unwrap_or_default();
    let legacy = legacy_save_path(&paths.root);
    if legacy.exists() && !save_files.iter().any(|path| path == &legacy) {
        save_files.push(legacy);
    }
    save_files
}
pub(crate) fn has_available_saves() -> bool {
    !available_save_files().is_empty()
}

fn start_option_count(lifecycle: &LifecycleState) -> usize {
    if start_has_load(lifecycle) {
        4
    } else {
        3
    }
}

fn start_has_load(lifecycle: &LifecycleState) -> bool {
    !lifecycle.save_files.is_empty()
}

fn start_options_index(lifecycle: &LifecycleState) -> usize {
    if start_has_load(lifecycle) {
        2
    } else {
        1
    }
}

impl LifecycleState {
    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn refresh_saves(&mut self) {
        self.save_files = available_save_files();
        self.selected = 0;
        self.dirty = true;
    }

    pub(crate) fn refresh_saved_games(&mut self) {
        self.refresh_saves();
    }

    pub(crate) fn start_new_game(&mut self, navigation: &mut NavigationState) {
        self.phase = LifecyclePhase::CreateCharacter;
        self.selected = 0;
        self.message = None;
        self.pending_state = None;
        navigation.return_screen = None;
        navigation.current_screen = Some(ScreenId::Lifecycle);
        navigation.selected = 0;
        self.dirty = true;
    }

    pub(crate) fn start_load_game(&mut self, navigation: &mut NavigationState) -> bool {
        self.refresh_saves();
        if self.save_files.is_empty() {
            self.message = None;
            self.dirty = false;
            return false;
        }

        self.phase = LifecyclePhase::Load;
        self.selected = 0;
        self.message = None;
        navigation.return_screen = None;
        navigation.current_screen = Some(ScreenId::Lifecycle);
        navigation.selected = 0;
        self.dirty = true;
        true
    }

    pub(crate) fn start_quit_confirmation(
        &mut self,
        navigation: &mut NavigationState,
        return_screen: Option<ScreenId>,
    ) {
        self.phase = LifecyclePhase::QuitConfirm;
        self.selected = 1;
        self.message = None;
        self.quit_variant = next_quit_variant();
        navigation.return_screen = return_screen;
        navigation.current_screen = Some(ScreenId::Lifecycle);
        navigation.selected = 1;
        self.dirty = true;
    }

    pub(crate) fn cancel_quit_confirmation(&mut self, navigation: &mut NavigationState) {
        let return_screen = navigation.return_screen.take();
        if return_screen == Some(ScreenId::Gameplay) && self.session.is_some() {
            self.phase = LifecyclePhase::Complete;
            self.selected = 0;
            self.message = None;
            navigation.current_screen = Some(ScreenId::Gameplay);
            navigation.selected = 0;
            self.dirty = false;
        } else {
            self.phase = LifecyclePhase::Start;
            self.selected = 0;
            self.message = None;
            navigation.current_screen = Some(ScreenId::Lifecycle);
            navigation.selected = 0;
            self.dirty = true;
        }
    }

    fn create_character(&mut self) {
        let world_name = if self.world_name.trim().is_empty() {
            "The Ashen Crown"
        } else {
            self.world_name.trim()
        };
        let character_name = if self.character_name.trim().is_empty() {
            "Wanderer"
        } else {
            self.character_name.trim()
        };
        let title = if self.character_title.trim().is_empty() {
            "Ash Walker"
        } else {
            self.character_title.trim()
        };
        let mut state = create_new_state(
            world_name,
            WorldMode::New,
            character_name.to_string(),
            title.to_string(),
        );
        crate::game::world::bootstrap_campaign_content(&mut state);
        let save_path = character_save_path(&GamePaths::get().saves_dir, character_name);
        self.session = Some(GameSession { state, save_path });
        self.pending_state = None;
        self.phase = LifecyclePhase::Complete;
        self.selected = 0;
        self.message = None;
        self.dirty = true;
    }

    fn finish_loading(&mut self) {
        if let Some((mut state, save_path)) = self.pending_state.take() {
            crate::game::world::bootstrap_campaign_content(&mut state);
            if state.character.alive {
                self.session = Some(GameSession { state, save_path });
                self.phase = LifecyclePhase::Complete;
            } else {
                self.session = Some(GameSession { state, save_path });
                self.phase = LifecyclePhase::Death;
            }
            self.selected = 0;
            self.dirty = true;
        }
    }

    fn inherit_character(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let name = if self.character_name.trim().is_empty() {
            "Heir"
        } else {
            self.character_name.trim()
        };
        let title = if self.character_title.trim().is_empty() {
            "Ash Walker"
        } else {
            self.character_title.trim()
        };
        let state = create_inherited_state(&session.state, name.to_string(), title.to_string());
        let save_path = character_save_path(&GamePaths::get().saves_dir, name);
        self.session = Some(GameSession { state, save_path });
        self.phase = LifecyclePhase::Complete;
        self.selected = 0;
        self.message = None;
        self.dirty = true;
    }
}

fn render_if_dirty(
    mut commands: Commands,
    mut lifecycle: ResMut<LifecycleState>,
    mut navigation: ResMut<NavigationState>,
    roots: Query<Entity, With<BevyScreenRoot>>,
) {
    if !lifecycle.dirty {
        return;
    }
    // The lifecycle state can change while another dedicated screen (such as
    // Options) is active. In that case, do not rebuild the lifecycle screen or
    // overwrite the destination selected by the input handler. The initial
    // None state is still accepted so Startup can render the start screen.
    if !should_render_lifecycle(navigation.current_screen) {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    render(&mut commands, &lifecycle);
    navigation.current_screen = Some(ScreenId::Lifecycle);
    navigation.selected = lifecycle.selected;
    lifecycle.dirty = false;
}

fn should_render_lifecycle(current_screen: Option<ScreenId>) -> bool {
    current_screen.is_none() || current_screen == Some(ScreenId::Lifecycle)
}

fn render(commands: &mut Commands, lifecycle: &LifecycleState) {
    let root = bevy_presentation::spawn_screen(commands, "THE ASHEN CHRONICLE");
    let panel = bevy_presentation::spawn_panel(commands, root);

    match lifecycle.phase {
        LifecyclePhase::Start => render_start(commands, panel, lifecycle),
        LifecyclePhase::Load => render_load(commands, panel, lifecycle),
        LifecyclePhase::CreateCharacter => render_creation(commands, panel, lifecycle),
        LifecyclePhase::QuitConfirm => render_quit(commands, panel, lifecycle),
        LifecyclePhase::Death => render_death(commands, panel, lifecycle),
        LifecyclePhase::Complete => render_complete(commands, panel, lifecycle),
    }
}

fn render_start(commands: &mut Commands, panel: Entity, lifecycle: &LifecycleState) {
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        "The road is quiet. Something is listening.",
    );
    if let Some(message) = &lifecycle.message {
        bevy_presentation::spawn_muted_label(commands, panel, message);
    }

    let actions: &[(&str, &str)] = if start_has_load(lifecycle) {
        &[
            ("✦", "New Game"),
            ("↺", "Load Game"),
            ("⚙", "Options"),
            ("×", "Quit"),
        ]
    } else {
        &[("✦", "New Game"), ("⚙", "Options"), ("×", "Quit")]
    };
    for (index, (icon, label)) in actions.iter().enumerate() {
        let button = bevy_presentation::spawn_action_button(commands, panel, index, *icon, *label);
        if lifecycle.selected == index {
            commands
                .entity(button)
                .insert(bevy_presentation::UiSelected);
        }
    }
}

fn render_load(commands: &mut Commands, panel: Entity, lifecycle: &LifecycleState) {
    bevy_presentation::spawn_muted_label(commands, panel, "Choose a life to continue.");
    if let Some(message) = &lifecycle.message {
        bevy_presentation::spawn_muted_label(commands, panel, message);
    }
    for (index, path) in lifecycle.save_files.iter().enumerate() {
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown save");
        let button = bevy_presentation::spawn_action_button(commands, panel, index, "▣", label);
        if lifecycle.selected == index {
            commands
                .entity(button)
                .insert(bevy_presentation::UiSelected);
        }
    }
    let back_index = lifecycle.save_files.len();
    let back = bevy_presentation::spawn_action_button(commands, panel, back_index, "←", "Back");
    if lifecycle.selected == back_index {
        commands.entity(back).insert(bevy_presentation::UiSelected);
    }
}

fn render_creation(commands: &mut Commands, panel: Entity, lifecycle: &LifecycleState) {
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        "Enter the names below. Enter advances to the next field.",
    );
    let fields = [
        ("World", &lifecycle.world_name),
        ("Character", &lifecycle.character_name),
        ("Title", &lifecycle.character_title),
    ];
    for (index, (label, value)) in fields.into_iter().enumerate() {
        bevy_presentation::spawn_text_input_field(
            commands,
            panel,
            index,
            label,
            value,
            lifecycle.selected == index,
        );
    }
    let button = bevy_presentation::spawn_action_button(commands, panel, 3, "▶", "Begin Life");
    if lifecycle.selected == 3 {
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
    }
}

#[derive(Debug, Clone, Copy)]
struct QuitVariant {
    question: &'static str,
    leave: &'static str,
    stay: &'static str,
    art: &'static str,
}

const QUIT_VARIANTS: [QuitVariant; 16] = [
    QuitVariant {
        question: "You can still hear your name after the room goes quiet.",
        leave: "Let it call.",
        stay: "Answer it.",
        art: r#"             .-.
            /   \
           |  () |
           |  /  |
            \\___/
              )
            (  (
             \\__)
"#,
    },
    QuitVariant {
        question: "The fire is dying. There are still pages left unread.",
        leave: "Close the book.",
        stay: "Turn the page.",
        art: r#"          /\
         /  \
        / /\ \
       / /  \ \
      /_/____\_\
        ||  ||
        ||  ||
        ||  ||
       _||__||_
"#,
    },
    QuitVariant {
        question: "Night has swallowed the road. Your footprints will not outlast it.",
        leave: "Let them fade.",
        stay: "Leave one more behind.",
        art: r#"       _..._       _..._
     .-'     '-. .-'     '-'.
    /           V           \
   /      _           _      \
   |     (_)         (_)     |
   |          .---.          |
    \        /     \        /
     '-._____'-----'_____.-'
"#,
    },
    QuitVariant {
        question: "The last ember is black. Beside it, the stone keeps its silence.",
        leave: "Let the silence stand.",
        stay: "Break the silence.",
        art: r#"            .
           / \
          /   \
         /_____\
         |     |
         | RIP |
         |     |
         |_____|
"#,
    },
    QuitVariant {
        question: "The gate is shut. Nothing beyond it is asking you to return.",
        leave: "Let it stay shut.",
        stay: "Open it once more.",
        art: r#"        ______________________
       /|                    |\
      / |                    | \
     /  |                    |  \
    /   |                    |   \
   /    |                    |    \
  /_____|____________________|_____\
        |                    |
        |        ____        |
        |       |    |       |
        |       |    |       |
        |_______|____|_______|
"#,
    },
    QuitVariant {
        question: "The flame is gone. You remember how the warmth felt.",
        leave: "Let the memory cool.",
        stay: "Strike one more spark.",
        art: r#"             /\
            /  \
           /____\
          |      |
          |  __  |
          | |  | |
          | |__| |
          |______|
             ||
          ___||___
         |        |
         |  .  .  |
         |________|
"#,
    },
    QuitVariant {
        question: "The road goes on without you. It always knew how.",
        leave: "Let it go without me.",
        stay: "Not without me.",
        art: r#"             /\                 /\
            /  \               /  \
           /    \             /    \
          /      \___________/      \
         /                         \
        /                           \
       /_____________________________\
                    ||
                    ||
                    ||
                    ||
"#,
    },
    QuitVariant {
        question: "The dead have patience. The living are the ones who run out.",
        leave: "Let the dead wait.",
        stay: "I am not finished.",
        art: r#"       _        _        _
      | |      | |      | |
     _| |__   _| |__   _| |__
    /     \  /     \  /     \
   /       \/       \/       \
        |     |     |
        |     |     |
   _____|_____|_____|_____
"#,
    },
    QuitVariant {
        question: "You were certain there was only one heartbeat.",
        leave: "Let the other one fade.",
        stay: "Listen again.",
        art: r#"      _    _      _    _
     / \\__/ \\____/ \\__/ \\         \\/       \\/
          \\______
"#,
    },
    QuitVariant {
        question: "There is still heat in the ashes, but it remembers no one.",
        leave: "Let it forget me.",
        stay: "Give it one more ember.",
        art: r#"             .-.
            (   )
             \ /
              V
          _.-'''-._
        .'         '.
       /   .     .   \
      |    |     |    |
       \   '.___.'   /
        '._       _.'
           '-----'
"#,
    },
    QuitVariant {
        question: "The mirror will not show you leaving.",
        leave: "Leave it behind.",
        stay: "Look again.",
        art: r#"       .-----------.
      /             \\
     |      /\\      |
     |     /  \\     |
     |    /____\\    |
     |               |
      \\             /
       '-----------'
"#,
    },
    QuitVariant {
        question: "The names on the stones are wearing thin.",
        leave: "Let them disappear.",
        stay: "Remember them a little longer.",
        art: r#"       __________   __________   __________
      |          | |          | |          |
      |  ....... | |  ....... | |  ....... |
      |  .....  | |  .....  | |  .....  |
      |__________| |__________| |__________|
"#,
    },
    QuitVariant {
        question: "The night took what it came for. One ember is still breathing.",
        leave: "Let it go quiet.",
        stay: "Keep it alive.",
        art: r#"          .-.
         (   )
          \ /
           |
        ___|___
       /       \
      /         \
     |  .----.  |
     | |      | |
      \ \____/ /
       '------'
"#,
    },
    QuitVariant {
        question: "You hear breathing. It stops when you do.",
        leave: "Keep walking.",
        stay: "Hold your breath.",
        art: r#"        .-""""-.
      .'  .--.  '.
     /   (    )   \\
     |    \\__/    |
      \\          /
       '---..---'
"#,
    },
    QuitVariant {
        question: "The rain washed the blood from the stones. Not the memory.",
        leave: "Let the rain finish the work.",
        stay: "Remember why it was spilled.",
        art: r#"          _________
         /________/|
        |        | |
        |   /\   | |
        |  /  \  | |
        | /____\ | /
        |________|/
           .  .
        .  .  .  .
"#,
    },
    QuitVariant {
        question: "You have been alone for a long time. Something disagrees.",
        leave: "Let it be wrong.",
        stay: "Let it speak.",
        art: r#"        .----.     .----.
       /      \\   /      \\
      |        | |        |
      |        | |        |
       \\______/   \\______/
"#,
    },
];

fn next_quit_variant() -> usize {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as usize)
        .unwrap_or(0);
    nanos % QUIT_VARIANTS.len()
}

fn render_quit(commands: &mut Commands, panel: Entity, lifecycle: &LifecycleState) {
    let variant = QUIT_VARIANTS
        .get(lifecycle.quit_variant % QUIT_VARIANTS.len())
        .expect("quit variant should exist");
    bevy_presentation::spawn_muted_label(commands, panel, variant.question);
    let art = commands
        .spawn((
            Text::new(variant.art),
            Node {
                width: percent(100),
                min_width: px(0),
                max_width: percent(100),
                flex_shrink: 1.0,
                ..default()
            },
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
            bevy_presentation::TextContent,
            TextFont::from_font_size(bevy_presentation::muted_font_size()),
            TextColor(bevy_presentation::THEME_TEXT),
        ))
        .id();
    commands.entity(panel).add_child(art);
    for (index, (icon, label)) in [("×", variant.leave), ("↩", variant.stay)]
        .into_iter()
        .enumerate()
    {
        let button = bevy_presentation::spawn_action_button(commands, panel, index, icon, label);
        if lifecycle.selected == index {
            commands
                .entity(button)
                .insert(bevy_presentation::UiSelected);
        }
    }
}

fn render_complete(commands: &mut Commands, panel: Entity, lifecycle: &LifecycleState) {
    let Some(session) = lifecycle.session.as_ref() else {
        bevy_presentation::spawn_muted_label(commands, panel, "No active life.");
        let button = bevy_presentation::spawn_action_button(commands, panel, 0, "×", "Quit");
        if lifecycle.selected == 0 {
            commands
                .entity(button)
                .insert(bevy_presentation::UiSelected);
        }
        return;
    };
    bevy_presentation::spawn_label(
        commands,
        panel,
        format!(
            "{} the {} is ready.",
            session.state.character.name, session.state.character.title
        ),
    );
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        "The Bevy gameplay flow will take over this session in the next migration step.",
    );
    if let Some(message) = &lifecycle.message {
        bevy_presentation::spawn_muted_label(commands, panel, message);
    }
    let button = bevy_presentation::spawn_action_button(commands, panel, 0, "×", "Quit");
    if lifecycle.selected == 0 {
        commands
            .entity(button)
            .insert(bevy_presentation::UiSelected);
    }
}

fn render_death(commands: &mut Commands, panel: Entity, lifecycle: &LifecycleState) {
    let Some(session) = lifecycle.session.as_ref() else {
        return;
    };
    let view = build_death_view(&session.state);
    bevy_presentation::spawn_label(commands, panel, view.screen.title);
    if let Some(subtitle) = view.screen.subtitle {
        bevy_presentation::spawn_muted_label(commands, panel, subtitle);
    }
    for line in view.screen.body {
        bevy_presentation::spawn_label(commands, panel, line);
    }
    bevy_presentation::spawn_muted_label(commands, panel, view.memory_note);
    for (index, (icon, label)) in [
        ("✦", "Create a new world"),
        ("↻", "Inherit this world with a new character"),
        ("×", "Quit"),
    ]
    .into_iter()
    .enumerate()
    {
        let button = bevy_presentation::spawn_action_button(commands, panel, index, icon, label);
        if lifecycle.selected == index {
            commands
                .entity(button)
                .insert(bevy_presentation::UiSelected);
        }
    }
}

fn build_death_view(state: &GameState) -> DeathView {
    let character = crate::presentation::CharacterView {
        name: state.character.name.clone(),
        title: state.character.title.clone(),
        hp: state.character.hp,
        max_hp: state.character.max_hp,
    };
    let location_name = state
        .world
        .location_by_id(state.character.location_id)
        .map(|location| location.name.clone())
        .unwrap_or_else(|| "an unknown place".to_string());
    let deeds = state
        .world
        .history
        .iter()
        .filter(|entry| {
            entry.text.contains(&character.display_name()) && entry.text.contains("completed ")
        })
        .map(|entry| entry.text.clone())
        .take(5)
        .collect::<Vec<_>>();
    let faction_standing = state
        .factions
        .iter()
        .map(|faction| FactionView {
            name: faction.name.clone(),
            reputation: faction.reputation,
            memories: Vec::new(),
        })
        .collect::<Vec<_>>();
    let dropped_items = state
        .corpses
        .last()
        .map(|corpse| {
            corpse
                .inventory
                .iter()
                .map(|item| ItemView {
                    id: item.id,
                    name: item.name.clone(),
                    description: item.description.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut body = vec![format!(
        "{} died at {} on turn {}.",
        character.display_name(),
        location_name,
        state.character.turn
    )];
    body.push(String::new());
    body.push("Deeds remembered:".to_string());
    if deeds.is_empty() {
        body.push("  None recorded.".to_string());
    } else {
        body.extend(deeds.iter().map(|deed| format!("  - {deed}")));
    }
    body.push(String::new());
    body.push("Faction standing at death:".to_string());
    if faction_standing.is_empty() {
        body.push("  None recorded.".to_string());
    } else {
        body.extend(
            faction_standing
                .iter()
                .map(|faction| format!("  - {} {:+}", faction.name, faction.reputation)),
        );
    }
    body.push(String::new());
    body.push("What remains on the body:".to_string());
    if dropped_items.is_empty() {
        body.push("  Nothing worth carrying.".to_string());
    } else {
        body.push(format!(
            "  {}",
            dropped_items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    DeathView {
        screen: ScreenView {
            title: "DEATH".to_string(),
            subtitle: Some("The body is still. The world is not.".to_string()),
            art: None,
            body,
        },
        character,
        location_name,
        turn: state.character.turn,
        deeds,
        faction_standing,
        dropped_items,
        memory_note: "The next life will know none of this as memory. It can only be discovered."
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn start_options_index_accounts_for_load_slot() {
        let mut lifecycle = super::LifecycleState::default();
        assert_eq!(super::start_option_count(&lifecycle), 3);
        assert_eq!(super::start_options_index(&lifecycle), 1);

        lifecycle
            .save_files
            .push(std::path::PathBuf::from("save.json"));
        assert_eq!(super::start_option_count(&lifecycle), 4);
        assert_eq!(super::start_options_index(&lifecycle), 2);
    }

    use super::*;

    #[test]
    fn quit_confirmation_can_return_to_gameplay() {
        let mut lifecycle = LifecycleState::default();
        let state = create_new_state(
            "Test World",
            WorldMode::New,
            "Tester".to_string(),
            "Ash Walker".to_string(),
        );
        lifecycle.session = Some(GameSession {
            state,
            save_path: PathBuf::from("saves/test.json.gz"),
        });
        lifecycle.phase = LifecyclePhase::QuitConfirm;
        lifecycle.selected = 1;

        let mut navigation = NavigationState {
            current_screen: Some(ScreenId::Lifecycle),
            return_screen: Some(ScreenId::Gameplay),
            selected: 1,
        };

        lifecycle.cancel_quit_confirmation(&mut navigation);

        assert_eq!(lifecycle.phase, LifecyclePhase::Complete);
        assert_eq!(navigation.current_screen, Some(ScreenId::Gameplay));
        assert_eq!(navigation.return_screen, None);
        assert_eq!(lifecycle.selected, 0);
        assert!(!lifecycle.dirty);
    }

    #[test]
    fn options_activation_keeps_navigation_on_the_options_screen() {
        let mut lifecycle = LifecycleState::default();
        let mut navigation = NavigationState {
            current_screen: Some(ScreenId::Lifecycle),
            return_screen: None,
            selected: 0,
        };

        lifecycle.phase = LifecyclePhase::Start;
        lifecycle.selected = 1;

        super::activate_selection(&mut lifecycle, &mut navigation);

        assert_eq!(navigation.current_screen, Some(ScreenId::Options));
        assert_eq!(navigation.return_screen, Some(ScreenId::Lifecycle));
    }

    #[test]
    fn load_back_returns_to_the_start_screen() {
        let mut lifecycle = LifecycleState {
            phase: LifecyclePhase::Load,
            ..LifecycleState::default()
        };
        lifecycle
            .save_files
            .push(std::path::PathBuf::from("saves/test.json.gz"));
        lifecycle.selected = lifecycle.save_files.len();
        let mut navigation = NavigationState {
            current_screen: Some(ScreenId::Lifecycle),
            return_screen: None,
            selected: lifecycle.selected,
        };

        super::activate_selection(&mut lifecycle, &mut navigation);

        assert_eq!(lifecycle.phase, LifecyclePhase::Start);
        assert_eq!(lifecycle.selected, 0);
        assert_eq!(navigation.current_screen, Some(ScreenId::Lifecycle));
        assert_eq!(navigation.selected, 0);
    }

    #[test]
    fn lifecycle_renderer_allows_initial_screen_but_not_dedicated_screen_overwrite() {
        assert!(super::should_render_lifecycle(None));
        assert!(super::should_render_lifecycle(Some(ScreenId::Lifecycle)));
        assert!(!super::should_render_lifecycle(Some(ScreenId::Options)));
        assert!(!super::should_render_lifecycle(Some(ScreenId::Gameplay)));
    }

    #[test]
    fn start_screen_has_expected_options_without_saves() {
        let state = LifecycleState::default();
        assert_eq!(start_option_count(&state), 3);
        assert!(!start_has_load(&state));
    }

    #[test]
    fn character_save_paths_use_the_dedicated_saves_directory() {
        let path = character_save_path(&GamePaths::get().saves_dir, "Ash Walker");
        assert_eq!(
            path,
            PathBuf::from("saves/ashen_chronicle_save_Ash Walker.json.gz")
        );
    }

    #[test]
    fn android_text_snapshots_replace_the_buffer_and_reset_the_cursor() {
        let mut field = EditableText::new("The Ashen Crown");

        field.queue_edit(TextEdit::Insert("stale pending edit".into()));
        super::replace_text_snapshot(&mut field, "Málaga 🜏");

        assert_eq!(field.value().to_string(), "Málaga 🜏");
        assert_eq!(field.pending_edits.len(), 1);
        assert_eq!(field.pending_edits[0], TextEdit::TextEnd(false));

        super::replace_text_snapshot(&mut field, "Short");

        assert_eq!(field.value().to_string(), "Short");
        assert_eq!(field.pending_edits.len(), 1);
        assert_eq!(field.pending_edits[0], TextEdit::TextEnd(false));
    }

    #[test]
    fn default_creation_values_match_legacy_flow() {
        let state = LifecycleState::default();
        assert_eq!(state.world_name, "The Ashen Crown");
        assert!(state.character_name.is_empty());
        assert_eq!(state.character_title, "Ash Walker");
    }
}
