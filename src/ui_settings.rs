//! Shared, persistent UI preferences for desktop and Android.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use atomic_write_file::AtomicWriteFile;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::game_paths::GamePaths;

const SETTINGS_FILE_NAME: &str = "ui_settings.json";
const DEFAULT_SCALE: f32 = 1.0;
const MIN_SCALE_PERCENT: i32 = 75;
const MAX_SCALE_PERCENT: i32 = 150;
const SCALE_STEP_PERCENT: i32 = 5;
pub(crate) const BASE_REM_SIZE_PX: f32 = 20.0;

/// User-controlled scale factors shared by desktop and Android.
#[derive(Resource, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct UiSettings {
    ui_scale: f32,
    font_scale: f32,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            ui_scale: DEFAULT_SCALE,
            font_scale: DEFAULT_SCALE,
        }
    }
}

impl UiSettings {
    pub(crate) fn load() -> Self {
        let path = Self::path();
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .unwrap_or_default()
            .sanitized()
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        let serialized = serde_json::to_vec_pretty(&self.sanitized())
            .map_err(io::Error::other)?;
        let mut file = AtomicWriteFile::open(Self::path())?;
        file.write_all(&serialized)?;
        file.commit()
    }

    pub(crate) fn ui_scale(&self) -> f32 {
        self.ui_scale
    }

    pub(crate) fn font_scale(&self) -> f32 {
        self.font_scale
    }

    pub(crate) fn rem_size_px(&self) -> f32 {
        BASE_REM_SIZE_PX * self.font_scale
    }

    pub(crate) fn adjust_ui_scale(&mut self, direction: i32) -> bool {
        adjust_scale(&mut self.ui_scale, direction)
    }

    pub(crate) fn adjust_font_scale(&mut self, direction: i32) -> bool {
        adjust_scale(&mut self.font_scale, direction)
    }

    pub(crate) fn reset(&mut self) -> bool {
        let defaults = Self::default();
        if *self == defaults {
            return false;
        }
        *self = defaults;
        true
    }

    fn sanitized(mut self) -> Self {
        self.ui_scale = sanitize_scale(self.ui_scale);
        self.font_scale = sanitize_scale(self.font_scale);
        self
    }

    fn path() -> PathBuf {
        GamePaths::get().config_dir.join(SETTINGS_FILE_NAME)
    }
}

fn adjust_scale(value: &mut f32, direction: i32) -> bool {
    if direction == 0 {
        return false;
    }

    let current_percent = (sanitize_scale(*value) * 100.0).round() as i32;
    let next_percent = (current_percent + direction.signum() * SCALE_STEP_PERCENT)
        .clamp(MIN_SCALE_PERCENT, MAX_SCALE_PERCENT);
    let next = next_percent as f32 / 100.0;
    if (*value - next).abs() < f32::EPSILON {
        return false;
    }

    *value = next;
    true
}

fn sanitize_scale(value: f32) -> f32 {
    if !value.is_finite() {
        return DEFAULT_SCALE;
    }
    (value.clamp(
        MIN_SCALE_PERCENT as f32 / 100.0,
        MAX_SCALE_PERCENT as f32 / 100.0,
    ) * 100.0)
        .round()
        / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_adjustments_respect_the_supported_range() {
        let mut settings = UiSettings::default();

        for _ in 0..20 {
            settings.adjust_ui_scale(1);
            settings.adjust_font_scale(1);
        }
        assert_eq!(settings.ui_scale(), 1.5);
        assert_eq!(settings.font_scale(), 1.5);

        for _ in 0..30 {
            settings.adjust_ui_scale(-1);
            settings.adjust_font_scale(-1);
        }
        assert_eq!(settings.ui_scale(), 0.75);
        assert_eq!(settings.font_scale(), 0.75);
    }

    #[test]
    fn reset_restores_both_default_scales() {
        let mut settings = UiSettings {
            ui_scale: 1.25,
            font_scale: 0.9,
        };

        assert!(settings.reset());
        assert_eq!(settings, UiSettings::default());
        assert!(!settings.reset());
    }

    #[test]
    fn invalid_and_out_of_range_values_are_sanitized() {
        assert_eq!(sanitize_scale(f32::NAN), DEFAULT_SCALE);
        assert_eq!(sanitize_scale(0.1), 0.75);
        assert_eq!(sanitize_scale(2.0), 1.5);
        assert_eq!(sanitize_scale(1.125), 1.13);
    }
}
