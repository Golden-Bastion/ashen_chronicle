//! Desktop-only window integration.

use std::io::Cursor;

use bevy::prelude::*;
use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};

const APP_ICON_BYTES: &[u8] = include_bytes!("../data/assets/icons/app/icon.png");

pub(crate) fn fullscreen_mode() -> WindowMode {
    WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current)
}

pub(crate) fn apply_window_icon(
    mut applied: Local<bool>,
    primary_window: Single<Entity, With<bevy::window::PrimaryWindow>>,
) {
    if *applied {
        return;
    }

    let Some(icon) = load_window_icon() else {
        return;
    };

    bevy::winit::WINIT_WINDOWS.with(|windows| {
        let windows = windows.borrow();
        let Some(window) = windows.get_window(*primary_window) else {
            return;
        };

        window.set_window_icon(Some(icon));
        *applied = true;
    });
}

fn load_window_icon() -> Option<winit::window::Icon> {
    let decoder = png::Decoder::new(Cursor::new(APP_ICON_BYTES));
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    let data = &buffer[..info.buffer_size()];

    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        png::ColorType::Grayscale => data
            .iter()
            .flat_map(|&gray| [gray, gray, gray, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        png::ColorType::Indexed => return None,
    };

    winit::window::Icon::from_rgba(rgba, info.width, info.height).ok()
}

#[cfg(test)]
#[path = "../tests/unit/desktop_window.rs"]
mod tests;
