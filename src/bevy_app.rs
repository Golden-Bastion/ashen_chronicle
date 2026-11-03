//! Bevy application bootstrap for the graphical frontend.
//!
//! Engine setup stays here; lifecycle and gameplay behavior remain in their
//! frontend-independent systems and Bevy adapters.

#[cfg(not(target_os = "android"))]
use crate::desktop_window;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::ui::{RemSize, UiScale};
#[cfg(not(target_os = "android"))]
use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};
use bevy_picking::events::PointerRelease;

use crate::{
    bevy_combat, bevy_console, bevy_feedback, bevy_gameplay, bevy_input_diagnostics,
    bevy_interactions, bevy_lifecycle, bevy_navigation_bridge, bevy_options, bevy_presentation,
    bevy_records, bevy_runtime, bevy_spatial, bevy_touch,
};

const WINDOW_TITLE: &str = "The Ashen Chronicle";

#[cfg(not(target_os = "android"))]
fn desktop_window_mode() -> WindowMode {
    WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current)
}

pub fn run() {
    let ui_settings = crate::ui_settings::UiSettings::load();
    let mut app = App::new();
    // EditableText reads PointerRelease even when no input-picking backend is
    // available. Register its message storage explicitly; touch gestures are
    // resolved by our adapter and suppressed before widget activation.
    app.add_message::<PointerRelease>();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: WINDOW_TITLE.to_owned(),
            resolution: (1280, 720).into(),
            #[cfg(not(target_os = "android"))]
            mode: desktop_window_mode(),
            resizable: true,
            ..default()
        }),
        ..default()
    }));
    app.insert_resource(UiScale(ui_settings.ui_scale()));
    app.insert_resource(RemSize(ui_settings.rem_size_px()));
    app.insert_resource(ui_settings);
    app.add_plugins(TabNavigationPlugin);
    bevy_presentation::install(&mut app);
    bevy_touch::install(&mut app);
    bevy_input_diagnostics::install(&mut app);
    bevy_runtime::install(&mut app);
    bevy_lifecycle::install(&mut app);
    bevy_gameplay::install(&mut app);
    bevy_spatial::install(&mut app);
    bevy_combat::install(&mut app);
    bevy_console::install(&mut app);
    bevy_navigation_bridge::install(&mut app);
    bevy_options::install(&mut app);
    bevy_records::install(&mut app);
    bevy_interactions::install(&mut app);
    bevy_feedback::install(&mut app);
    #[cfg(not(target_os = "android"))]
    app.add_systems(Update, desktop_window::apply_window_icon);
    app.add_systems(Startup, setup).run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[cfg(test)]
#[cfg(not(target_os = "android"))]
mod tests {
    use super::WINDOW_TITLE;
    use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};

    #[test]
    fn foundation_window_configuration_is_stable() {
        assert_eq!(WINDOW_TITLE, "The Ashen Chronicle");
        assert_eq!(
            desktop_window_mode(),
            WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current)
        );
    }
}
