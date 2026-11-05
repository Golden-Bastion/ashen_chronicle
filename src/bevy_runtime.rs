//! Platform runtime lifecycle state for the Bevy frontend.
//!
//! This records application pause/resume and window focus transitions without
//! changing authoritative gameplay state or introducing save-on-background behavior.

use bevy::prelude::*;
use bevy::window::{AppLifecycle, WindowEvent};

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct RuntimeLifecycleState {
    pub(crate) app_lifecycle: AppLifecycle,
    pub(crate) window_focused: bool,
}

impl Default for RuntimeLifecycleState {
    fn default() -> Self {
        Self {
            app_lifecycle: AppLifecycle::Running,
            window_focused: true,
        }
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<RuntimeLifecycleState>()
        .add_systems(Update, track_window_lifecycle);
}

fn track_window_lifecycle(
    mut events: MessageReader<WindowEvent>,
    mut state: ResMut<RuntimeLifecycleState>,
) {
    for event in events.read() {
        match event {
            WindowEvent::AppLifecycle(lifecycle) => {
                state.app_lifecycle = *lifecycle;
            }
            WindowEvent::WindowFocused(focus) => {
                state.window_focused = focus.focused;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/bevy_runtime.rs"]
mod tests;
