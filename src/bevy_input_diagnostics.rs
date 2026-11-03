//! Android input diagnostics for device-side event delivery.

use bevy::prelude::*;

#[cfg(target_os = "android")]
use bevy::input::keyboard::KeyboardInput;

#[cfg(target_os = "android")]
use std::io::Write;

#[cfg(target_os = "android")]
pub fn install(app: &mut App) {
    app.add_systems(
        Update,
        (log_ime_events, log_keyboard_text, log_browser_back),
    );
}

#[cfg(not(target_os = "android"))]
pub fn install(_app: &mut App) {}

#[cfg(target_os = "android")]
fn log_ime_events(mut ime: MessageReader<Ime>) {
    for event in ime.read() {
        write_debug_line(&format!("Android IME event: {event:?}"));
    }
}

#[cfg(target_os = "android")]
fn log_keyboard_text(mut keyboard: MessageReader<KeyboardInput>) {
    for event in keyboard.read() {
        if event.text.as_ref().is_some_and(|text| !text.is_empty()) {
            write_debug_line(&format!(
                "Android KeyboardInput text: {:?} logical_key={:?}",
                event.text, event.logical_key
            ));
        }
    }
}

#[cfg(target_os = "android")]
fn log_browser_back(keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::BrowserBack) {
        write_debug_line("Android BrowserBack key event received");
    }
}

#[cfg(target_os = "android")]
fn write_debug_line(line: &str) {
    // Diagnostics remain inside the app-private save area; no shared-folder
    // mirroring or working-directory assumption is involved.
    bevy::log::info!("{line}");
    let path = crate::game_paths::GamePaths::get()
        .saves_dir
        .join("_input_debug.log");
    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = result {
        bevy::log::warn!("Could not write input diagnostics to {path:?}: {error}");
    }
}
