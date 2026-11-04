//! Options-owned save import/export and mod management workflows.

use std::path::PathBuf;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::bevy_gameplay::GameplayState;
use crate::bevy_lifecycle::{LifecyclePhase, LifecycleState};
use crate::bevy_presentation::{
    self, BevyScreenRoot, GameplayInputQueue, NavigationState, ScreenId, UiSelected,
};
use crate::input::InputEvent;
use crate::mod_manager::InstalledMod;

const MOD_ACTIONS_PER_MOD: usize = 4;
const MOD_ENABLE_ACTION: usize = 0;
const MOD_DELETE_ACTION: usize = 1;
const MOD_PRIORITY_UP_ACTION: usize = 2;
const MOD_PRIORITY_DOWN_ACTION: usize = 3;

fn mod_action_start_index(mod_index: usize) -> usize {
    mod_index * MOD_ACTIONS_PER_MOD
}

fn mod_install_action_index(mod_count: usize) -> usize {
    mod_count * MOD_ACTIONS_PER_MOD
}

fn mod_back_action_index(mod_count: usize) -> usize {
    mod_install_action_index(mod_count) + 1
}

#[derive(Resource)]
pub(crate) struct OptionsState {
    selected: usize,
    tab: usize,
    message: Option<String>,
    active: bool,
    dirty: bool,
    saves: Vec<PathBuf>,
    mods: Vec<InstalledMod>,
    pending_restart: bool,
    pending_remove_id: Option<String>,
    #[cfg(target_os = "android")]
    last_storage_poll: f32,
    #[cfg(target_os = "android")]
    processed_imports: std::collections::HashSet<PathBuf>,
}

impl Default for OptionsState {
    fn default() -> Self {
        Self {
            selected: 0,
            tab: 0,
            message: None,
            active: false,
            dirty: true,
            saves: Vec::new(),
            mods: Vec::new(),
            pending_restart: false,
            pending_remove_id: None,
            #[cfg(target_os = "android")]
            last_storage_poll: 0.0,
            #[cfg(target_os = "android")]
            processed_imports: std::collections::HashSet::new(),
        }
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<OptionsState>().add_systems(
        Update,
        (
            sync_screen,
            refresh_storage,
            options_input,
            render_if_active,
        )
            .chain(),
    );
}

fn sync_screen(
    lifecycle: Res<LifecycleState>,
    navigation: Res<NavigationState>,
    mut options: ResMut<OptionsState>,
) {
    let active = matches!(
        lifecycle.phase,
        LifecyclePhase::Start | LifecyclePhase::Complete
    ) && navigation.current_screen == Some(ScreenId::Options);

    if active && !options.active {
        options.active = true;
        options.selected = 0;
        options.tab = 0;
        options.message = None;
        options.pending_remove_id = None;
        refresh_catalogs(&mut options);
        #[cfg(target_os = "android")]
        {
            options.last_storage_poll = 0.0;
        }
        options.dirty = true;
    } else if !active {
        options.active = false;
    }
}

fn refresh_catalogs(options: &mut OptionsState) {
    let paths = crate::game_paths::GamePaths::get();
    options.saves = crate::persistence::find_save_files(&paths.saves_dir).unwrap_or_default();
    let legacy = crate::persistence::legacy_save_path(&paths.root);
    if legacy.is_file() && !options.saves.iter().any(|path| path == &legacy) {
        options.saves.push(legacy);
    }
    options.mods = match crate::mod_manager::list_installed_mods() {
        Ok(mods) => mods,
        Err(error) => {
            options.message = Some(format!("Could not read installed mods: {error}"));
            Vec::new()
        }
    };
    let maximum_selection = options_selection_end(options);
    options.selected = options.selected.min(maximum_selection);
}

#[cfg(target_os = "android")]
fn refresh_storage(
    time: Res<Time>,
    navigation: Res<NavigationState>,
    mut options: ResMut<OptionsState>,
    mut lifecycle: ResMut<LifecycleState>,
) {
    if navigation.current_screen != Some(ScreenId::Options) {
        return;
    }
    let elapsed = time.elapsed_secs();
    if elapsed - options.last_storage_poll < 0.5 {
        return;
    }
    options.last_storage_poll = elapsed;

    let paths = crate::game_paths::GamePaths::get();
    let mut changed = false;
    for (directory, kind) in [
        (&paths.save_imports_dir, "save"),
        (&paths.mod_imports_dir, "mod"),
    ] {
        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                options.message = Some(format!("Could not inspect imported files: {error}"));
                options.dirty = true;
                continue;
            }
        };
        for entry in entries.flatten() {
            let source = entry.path();
            if !source.is_file() || options.processed_imports.contains(&source) {
                continue;
            }
            let result = match kind {
                "save" if source.extension().and_then(|value| value.to_str()) == Some("save") => {
                    crate::persistence::install_imported_save(&source).map(|_| ())
                }
                "mod" if source.extension().and_then(|value| value.to_str()) == Some("zip") => {
                    crate::mod_manager::install_mod_archive(&source).map(|_| ())
                }
                _ => continue,
            };
            options.processed_imports.insert(source.clone());
            match result {
                Ok(()) => {
                    let _ = std::fs::remove_file(&source);
                    options.message = Some(if kind == "save" {
                        "Save imported successfully.".to_string()
                    } else {
                        options.pending_restart = true;
                        "Mod installed. Restart the game to activate it.".to_string()
                    });
                    changed = true;
                }
                Err(error) => {
                    options.message =
                        Some(format!("Could not import {}: {error}", source.display()));
                    options.dirty = true;
                }
            }
        }
    }
    if changed {
        refresh_catalogs(&mut options);
        lifecycle.refresh_saved_games();
        options.dirty = true;
    }
}

#[cfg(not(target_os = "android"))]
fn refresh_storage(
    _time: Res<Time>,
    _navigation: Res<NavigationState>,
    _options: ResMut<OptionsState>,
    _lifecycle: ResMut<LifecycleState>,
) {
}

fn options_input(
    mut lifecycle: ResMut<LifecycleState>,
    mut navigation: ResMut<NavigationState>,
    mut options: ResMut<OptionsState>,
    mut ui_settings: ResMut<crate::ui_settings::UiSettings>,
    gameplay: Res<GameplayState>,
    mut input_queue: ResMut<GameplayInputQueue>,
) {
    if !matches!(
        lifecycle.phase,
        LifecyclePhase::Start | LifecyclePhase::Complete
    ) || navigation.current_screen != Some(ScreenId::Options)
        || input_queue.0.is_empty()
    {
        return;
    }

    let events = std::mem::take(&mut input_queue.0);
    for event in events {
        match event {
            InputEvent::Up => {
                options.selected = options.selected.saturating_sub(1);
                options.pending_remove_id = None;
            }
            InputEvent::Down => {
                options.selected = (options.selected + 1).min(options_selection_end(&options));
                options.pending_remove_id = None;
            }
            InputEvent::Home => {
                options.selected = 0;
                options.pending_remove_id = None;
            }
            InputEvent::End => {
                options.selected = options_selection_end(&options);
                options.pending_remove_id = None;
            }
            InputEvent::SelectTab(tab) if tab < 3 => {
                options.tab = tab;
                options.selected = 0;
                options.message = None;
                options.pending_remove_id = None;
            }
            InputEvent::Tab => {
                options.tab = (options.tab + 1) % 3;
                options.selected = 0;
                options.message = None;
                options.pending_remove_id = None;
            }
            InputEvent::Confirm => match options.tab {
                0 => {
                    activate_saves_action(&mut options, &mut lifecycle, &mut navigation, &gameplay)
                }
                1 => activate_mods_action(&mut options, &mut lifecycle, &mut navigation, &gameplay),
                2 => activate_interface_action(
                    &mut options,
                    &mut ui_settings,
                    &mut lifecycle,
                    &mut navigation,
                    &gameplay,
                ),
                _ => {}
            },
            InputEvent::Cancel => {
                close_options(&mut lifecycle, &mut navigation, &mut options, &gameplay)
            }
            _ => {}
        }
        options.dirty = true;
    }
    navigation.selected = options.selected;
}

fn options_selection_end(options: &OptionsState) -> usize {
    match options.tab {
        0 => options.saves.len() + 1,
        1 => mod_back_action_index(options.mods.len()),
        2 => 5,
        _ => 0,
    }
}

fn activate_saves_action(
    options: &mut OptionsState,
    lifecycle: &mut LifecycleState,
    navigation: &mut NavigationState,
    gameplay: &GameplayState,
) {
    if options.selected < options.saves.len() {
        let path = options.saves[options.selected].clone();
        export_save(options, &path);
    } else if options.selected == options.saves.len() {
        import_save(options, lifecycle);
    } else {
        close_options(lifecycle, navigation, options, gameplay);
    }
}

fn activate_mods_action(
    options: &mut OptionsState,
    lifecycle: &mut LifecycleState,
    navigation: &mut NavigationState,
    gameplay: &GameplayState,
) {
    if options.selected == mod_install_action_index(options.mods.len()) {
        import_mod(options);
        return;
    }
    if options.selected == mod_back_action_index(options.mods.len()) {
        close_options(lifecycle, navigation, options, gameplay);
        return;
    }

    let mod_index = options.selected / MOD_ACTIONS_PER_MOD;
    let action = options.selected % MOD_ACTIONS_PER_MOD;
    let Some(item) = options.mods.get(mod_index).cloned() else {
        return;
    };

    let result = match action {
        MOD_ENABLE_ACTION => crate::mod_manager::set_mod_enabled(&item.manifest.id, !item.enabled),
        MOD_DELETE_ACTION => {
            if options.pending_remove_id.as_deref() != Some(item.manifest.id.as_str()) {
                options.pending_remove_id = Some(item.manifest.id.clone());
                options.message = Some(format!(
                    "Select Confirm again to permanently remove {}.",
                    item.manifest.name
                ));
                return;
            }
            options.pending_remove_id = None;
            crate::mod_manager::remove_mod(&item.manifest.id)
        }
        MOD_PRIORITY_UP_ACTION => {
            crate::mod_manager::set_mod_priority(&item.manifest.id, item.priority.saturating_add(1))
        }
        MOD_PRIORITY_DOWN_ACTION => {
            crate::mod_manager::set_mod_priority(&item.manifest.id, item.priority.saturating_sub(1))
        }
        _ => return,
    };

    match result {
        Ok(()) => {
            options.pending_restart = true;
            options.message = Some(format!(
                "Updated {}. Restart the game to apply mod changes.",
                item.manifest.name
            ));
            refresh_catalogs(options);
        }
        Err(error) => {
            options.message = Some(format!(
                "Could not update mod {}: {error}",
                item.manifest.id
            ))
        }
    }
}

fn activate_interface_action(
    options: &mut OptionsState,
    settings: &mut crate::ui_settings::UiSettings,
    lifecycle: &mut LifecycleState,
    navigation: &mut NavigationState,
    gameplay: &GameplayState,
) {
    let changed = match options.selected {
        0 => settings.adjust_ui_scale(-crate::ui_settings::SETTING_STEP),
        1 => settings.adjust_ui_scale(crate::ui_settings::SETTING_STEP),
        2 => settings.adjust_font_size(-crate::ui_settings::SETTING_STEP),
        3 => settings.adjust_font_size(crate::ui_settings::SETTING_STEP),
        4 => {
            settings.reset();
            true
        }
        5 => {
            close_options(lifecycle, navigation, options, gameplay);
            return;
        }
        _ => false,
    };

    if changed {
        match settings.save() {
            Ok(()) => {
                options.message = Some(format!(
                    "UI scale: {}%. Font size: {}%.",
                    settings.ui_scale_percent(),
                    settings.font_size_percent()
                ));
            }
            Err(error) => {
                options.message = Some(format!("Could not save interface settings: {error}"));
            }
        }
    } else {
        options.message = Some("The selected setting is already at its limit.".to_string());
    }
}

fn import_save(options: &mut OptionsState, _lifecycle: &mut LifecycleState) {
    #[cfg(not(target_os = "android"))]
    {
        let Some(source) = rfd::FileDialog::new()
            .set_title("Import a save")
            .add_filter("Ashen Chronicle saves", &["gz", "json", "save"])
            .pick_file()
        else {
            options.message = Some("Save import cancelled.".to_string());
            return;
        };
        match crate::persistence::install_imported_save(&source) {
            Ok(target) => {
                options.message = Some(format!(
                    "Imported save as {}.",
                    target.file_name().unwrap_or_default().to_string_lossy()
                ));
                refresh_catalogs(options);
                _lifecycle.refresh_saved_games();
            }
            Err(error) => options.message = Some(format!("Could not import save: {error}")),
        }
    }
    #[cfg(target_os = "android")]
    {
        if android_request_storage_import("saves") {
            options.message =
                Some("Choose a save file in the Android document picker.".to_string());
        } else {
            options.message = Some("The Android document picker is unavailable.".to_string());
        }
    }
}

fn export_save(options: &mut OptionsState, source: &std::path::Path) {
    if let Err(error) = crate::persistence::validate_save(source) {
        options.message = Some(format!("Could not export invalid save: {error}"));
        return;
    }
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "ashen_chronicle_save.json.gz".to_string());
    #[cfg(not(target_os = "android"))]
    {
        let Some(destination) = rfd::FileDialog::new()
            .set_title("Export save")
            .set_file_name(&file_name)
            .save_file()
        else {
            options.message = Some("Save export cancelled.".to_string());
            return;
        };
        match crate::persistence::export_save(source, &destination) {
            Ok(()) => {
                options.message = Some(format!("Exported save to {}.", destination.display()))
            }
            Err(error) => options.message = Some(format!("Could not export save: {error}")),
        }
    }
    #[cfg(target_os = "android")]
    {
        if android_request_storage_export(source, &file_name, "application/gzip") {
            options.message = Some("Choose a destination for the save export.".to_string());
        } else {
            options.message = Some("The Android document picker is unavailable.".to_string());
        }
    }
}

fn import_mod(options: &mut OptionsState) {
    #[cfg(not(target_os = "android"))]
    {
        let Some(source) = rfd::FileDialog::new()
            .set_title("Install a mod ZIP")
            .add_filter("ZIP archives", &["zip"])
            .pick_file()
        else {
            options.message = Some("Mod installation cancelled.".to_string());
            return;
        };
        match crate::mod_manager::install_mod_archive(&source) {
            Ok(item) => {
                options.pending_restart = true;
                options.message = Some(format!(
                    "Installed {}. Restart the game to activate it.",
                    item.manifest.name
                ));
                refresh_catalogs(options);
            }
            Err(error) => options.message = Some(format!("Could not install mod: {error}")),
        }
    }
    #[cfg(target_os = "android")]
    {
        if android_request_storage_import("mods") {
            options.message = Some("Choose a mod ZIP in the Android document picker.".to_string());
        } else {
            options.message = Some("The Android document picker is unavailable.".to_string());
        }
    }
}

fn close_options(
    lifecycle: &mut LifecycleState,
    navigation: &mut NavigationState,
    options: &mut OptionsState,
    gameplay: &GameplayState,
) {
    let return_screen = navigation.return_screen.take();
    lifecycle.refresh_saved_games();
    options.active = false;
    options.selected = 0;
    options.tab = 0;
    options.message = None;
    options.pending_remove_id = None;
    options.dirty = true;
    match return_screen {
        Some(ScreenId::Lifecycle) => {
            navigation.current_screen = Some(ScreenId::Lifecycle);
            navigation.selected = lifecycle.selected;
            lifecycle.mark_dirty();
        }
        _ => {
            navigation.current_screen = Some(ScreenId::Gameplay);
            navigation.selected = gameplay.navigation_selection();
        }
    }
}

fn render_if_active(
    mut commands: Commands,
    lifecycle: Res<LifecycleState>,
    navigation: Res<NavigationState>,
    mut options: ResMut<OptionsState>,
    ui_settings: Res<crate::ui_settings::UiSettings>,
    roots: Query<Entity, With<BevyScreenRoot>>,
) {
    if !matches!(
        lifecycle.phase,
        LifecyclePhase::Start | LifecyclePhase::Complete
    ) || navigation.current_screen != Some(ScreenId::Options)
        || !options.active
        || !options.dirty
    {
        return;
    }

    for root in &roots {
        commands.entity(root).despawn();
    }

    let root = bevy_presentation::spawn_screen(&mut commands, "OPTIONS");
    let panel = bevy_presentation::spawn_fixed_panel(&mut commands, root);
    commands.entity(panel).insert((
        bevy_presentation::UiSurface,
        BorderColor::all(bevy_presentation::THEME_BORDER),
        BackgroundColor(bevy_presentation::THEME_SURFACE_STRONG),
    ));
    bevy_presentation::spawn_muted_label(
        &mut commands,
        panel,
        "Saves and mods are stored privately on this device. Use import/export to move them.",
    );

    let tab_strip = commands
        .spawn(Node {
            width: percent(100),
            min_width: px(0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: bevy_presentation::responsive_compact_gap(),
            row_gap: px(4),
            flex_shrink: 0.0,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(tab_strip);
    commands.entity(tab_strip).with_children(|tabs| {
        for (index, label) in [(0, "Saves"), (1, "Mods"), (2, "Interface")] {
            let mut tab = tabs.spawn((
                Button,
                Hovered::default(),
                bevy_presentation::UiTouchTargetShape::MinimumHeight,
                bevy_presentation::ChoiceButton { index },
                bevy_presentation::UiTabButton { index },
                bevy_presentation::UiStyledButton,
                Node {
                    width: Val::Auto,
                    min_width: px(100),
                    min_height: bevy_presentation::touch_target_size(),
                    padding: UiRect::axes(bevy_presentation::responsive_surface_padding(), px(6)),
                    border: UiRect::all(px(1)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    ..default()
                },
                BorderColor::all(bevy_presentation::THEME_BORDER),
                BackgroundColor(bevy_presentation::THEME_SURFACE),
                children![(
                    Text::new(label),
                    bevy_presentation::TextContent,
                    TextFont::from_font_size(bevy_presentation::muted_font_size()),
                    TextColor(bevy_presentation::THEME_TEXT),
                )],
            ));
            if options.tab == index {
                tab.insert(UiSelected);
            }
        }
    });

    let list = bevy_presentation::spawn_scrollable_list(&mut commands, panel);
    let footer = bevy_presentation::spawn_option_footer(&mut commands, panel);
    match options.tab {
        0 => render_saves_tab(&mut commands, list, footer, &options),
        1 => render_mods_tab(&mut commands, list, footer, &options),
        2 => render_interface_tab(&mut commands, list, footer, &options, &ui_settings),
        _ => {}
    }

    if options.pending_restart {
        bevy_presentation::spawn_context_message(
            &mut commands,
            panel,
            "Mod changes are pending. Restart the game to apply them.",
        );
    }
    if let Some(message) = &options.message {
        bevy_presentation::spawn_context_message(&mut commands, panel, message);
    }
    options.dirty = false;
}

fn render_saves_tab(
    commands: &mut Commands,
    list: Entity,
    footer: Entity,
    options: &OptionsState,
) {
    bevy_presentation::spawn_label(commands, list, "SAVES");
    bevy_presentation::spawn_muted_label(
        commands,
        list,
        "Export an existing save or import one from a file. Imports never overwrite an existing slot.",
    );
    if options.saves.is_empty() {
        bevy_presentation::spawn_muted_label(commands, list, "No saves found.");
    }
    for (index, save) in options.saves.iter().enumerate() {
        let name = save
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| save.display().to_string());
        let row = bevy_presentation::spawn_option_row(commands, list);
        bevy_presentation::spawn_label(commands, row.label, name);
        spawn_option_control_button(
            commands,
            row.controls,
            index,
            "Export",
            options.selected == index,
        );
    }
    let import_index = options.saves.len();
    spawn_option_control_button(
        commands,
        footer,
        import_index,
        "Import save",
        options.selected == import_index,
    );
    spawn_option_control_button(
        commands,
        footer,
        import_index + 1,
        "Back",
        options.selected == import_index + 1,
    );
}

fn render_mods_tab(
    commands: &mut Commands,
    list: Entity,
    footer: Entity,
    options: &OptionsState,
) {
    bevy_presentation::spawn_label(commands, list, "MOD MANAGEMENT");
    bevy_presentation::spawn_muted_label(
        commands,
        list,
        "Enable or disable mods, adjust load priority, or remove installed mods. Changes require restart.",
    );
    if options.mods.is_empty() {
        bevy_presentation::spawn_muted_label(commands, list, "No mods installed.");
    }
    for (mod_index, item) in options.mods.iter().enumerate() {
        let row = bevy_presentation::spawn_option_row(commands, list);
        bevy_presentation::spawn_label(commands, row.label, &item.manifest.name);
        bevy_presentation::spawn_muted_label(
            commands,
            row.label,
            format!(
                "id={} · version={} · priority={} · assets={}",
                item.manifest.id, item.manifest.version, item.priority, item.asset_count
            ),
        );

        let base = mod_action_start_index(mod_index);
        spawn_option_control_button(
            commands,
            row.controls,
            base + MOD_ENABLE_ACTION,
            if item.enabled { "Disable" } else { "Enable" },
            options.selected == base + MOD_ENABLE_ACTION,
        );
        spawn_option_control_button(
            commands,
            row.controls,
            base + MOD_DELETE_ACTION,
            if options.pending_remove_id.as_deref() == Some(item.manifest.id.as_str()) {
                "Confirm"
            } else {
                "Delete"
            },
            options.selected == base + MOD_DELETE_ACTION,
        );
        spawn_option_control_button(
            commands,
            row.controls,
            base + MOD_PRIORITY_UP_ACTION,
            "Up",
            options.selected == base + MOD_PRIORITY_UP_ACTION,
        );
        spawn_option_control_button(
            commands,
            row.controls,
            base + MOD_PRIORITY_DOWN_ACTION,
            "Down",
            options.selected == base + MOD_PRIORITY_DOWN_ACTION,
        );
    }
    let install_index = mod_install_action_index(options.mods.len());
    spawn_option_control_button(
        commands,
        footer,
        install_index,
        "Install mod ZIP",
        options.selected == install_index,
    );
    spawn_option_control_button(
        commands,
        footer,
        install_index + 1,
        "Back",
        options.selected == install_index + 1,
    );
}

fn spawn_option_control_button(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    label: impl Into<String>,
    selected: bool,
) {
    let button = bevy_presentation::spawn_compact_action_button(commands, parent, index, label);
    if selected {
        commands.entity(button).insert(UiSelected);
    }
}

fn spawn_option_button(
    commands: &mut Commands,
    panel: Entity,
    index: usize,
    label: impl Into<String>,
    selected: bool,
) {
    let button = bevy_presentation::spawn_action_button(commands, panel, index, "storage", label);
    if selected {
        commands.entity(button).insert(UiSelected);
    }
}

fn render_interface_tab(
    commands: &mut Commands,
    panel: Entity,
    options: &OptionsState,
    settings: &crate::ui_settings::UiSettings,
) {
    bevy_presentation::spawn_label(commands, panel, "INTERFACE");
    bevy_presentation::spawn_muted_label(
        commands,
        panel,
        "Adjust interface layout and text independently. Changes apply immediately and are saved on this device.",
    );

    for (label, value, decrease, increase) in [
        ("UI scale", settings.ui_scale_percent(), 0, 1),
        ("Font size", settings.font_size_percent(), 2, 3),
    ] {
        let row = bevy_presentation::spawn_option_row(
            commands,
            panel,
            bevy_presentation::OptionRowLabelSide::Right,
        );
        spawn_option_control_button(
            commands,
            row.controls,
            decrease,
            "−5%",
            options.selected == decrease,
        );
        spawn_option_control_button(
            commands,
            row.controls,
            increase,
            "+5%",
            options.selected == increase,
        );
        bevy_presentation::spawn_option_row_value(
            commands,
            row.label,
            format!("{label}: {value}%"),
            bevy_presentation::OptionRowLabelSide::Right,
        );
    }

    let actions = bevy_presentation::spawn_option_row(
        commands,
        panel,
        bevy_presentation::OptionRowLabelSide::Right,
    );
    spawn_option_control_button(
        commands,
        actions.controls,
        4,
        "Reset",
        options.selected == 4,
    );
    spawn_option_control_button(commands, actions.controls, 5, "Back", options.selected == 5);
    bevy_presentation::spawn_option_row_value(
        commands,
        actions.label,
        "Other actions",
        bevy_presentation::OptionRowLabelSide::Right,
    );
}

#[cfg(target_os = "android")]
fn android_request_storage_import(kind: &str) -> bool {
    let Some(app) = bevy::android::ANDROID_APP.get().cloned() else {
        return false;
    };
    let kind = kind.to_owned();
    let java_app = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(java_app.vm_as_ptr().cast()) };
        if let Err(error) = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let raw_activity = java_app.activity_as_ptr() as jni::sys::jobject;
            let activity = unsafe {
                env.as_cast_raw::<jni::refs::Global<jni::objects::JObject>>(&raw_activity)?
            };
            let value = env.new_string(&kind)?;
            env.call_method(
                activity.as_ref(),
                jni::jni_str!("requestStorageImport"),
                jni::jni_sig!("(Ljava/lang/String;)V"),
                &[jni::JValue::from(&value)],
            )?;
            Ok(())
        }) {
            bevy::log::warn!("Could not open Android document picker: {error}");
        }
    }));
    true
}

#[cfg(target_os = "android")]
fn android_request_storage_export(
    source: &std::path::Path,
    file_name: &str,
    mime_type: &str,
) -> bool {
    let Some(app) = bevy::android::ANDROID_APP.get().cloned() else {
        return false;
    };
    let source = source.to_string_lossy().to_string();
    let file_name = file_name.to_owned();
    let mime_type = mime_type.to_owned();
    let java_app = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(java_app.vm_as_ptr().cast()) };
        if let Err(error) = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let raw_activity = java_app.activity_as_ptr() as jni::sys::jobject;
            let activity = unsafe {
                env.as_cast_raw::<jni::refs::Global<jni::objects::JObject>>(&raw_activity)?
            };
            let source = env.new_string(&source)?;
            let file_name = env.new_string(&file_name)?;
            let mime_type = env.new_string(&mime_type)?;
            env.call_method(
                activity.as_ref(),
                jni::jni_str!("requestStorageExport"),
                jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
                &[
                    jni::JValue::from(&source),
                    jni::JValue::from(&file_name),
                    jni::JValue::from(&mime_type),
                ],
            )?;
            Ok(())
        }) {
            bevy::log::warn!("Could not open Android export picker: {error}");
        }
    }));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_tabs_have_three_categories() {
        let mut options = OptionsState::default();
        assert_eq!(options_selection_end(&options), 1); // Import and Back.
        options.tab = 1;
        assert_eq!(options_selection_end(&options), 1); // Install and Back.
        options.tab = 2;
        assert_eq!(options_selection_end(&options), 5);
    }

    #[test]
    fn mod_action_slots_match_the_row_control_order() {
        let second_mod_first_action = mod_action_start_index(1);
        assert_eq!(MOD_ENABLE_ACTION, 0);
        assert_eq!(MOD_DELETE_ACTION, 1);
        assert_eq!(MOD_PRIORITY_UP_ACTION, 2);
        assert_eq!(MOD_PRIORITY_DOWN_ACTION, 3);
        assert_eq!(second_mod_first_action, 5);
        assert_eq!(
            second_mod_first_action + MOD_PRIORITY_DOWN_ACTION,
            second_mod_first_action + MOD_ACTIONS_PER_MOD - 1
        );
    }
}
