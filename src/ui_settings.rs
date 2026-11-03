use std::fs;
use std::io::{self, Write};

use atomic_write_file::AtomicWriteFile;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const SETTINGS_FILE_NAME: &str = "ui_settings.json";
const MIN_UI_SCALE_PERCENT: u8 = 75;
const MAX_UI_SCALE_PERCENT: u8 = 125;
const MIN_FONT_SIZE_PERCENT: u8 = 75;
const MAX_FONT_SIZE_PERCENT: u8 = 150;
pub(crate) const SETTING_STEP: i16 = 5;
const BASE_REM_VMIN_PERCENT: f32 = 0.02778;

#[derive(Resource, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct UiSettings {
    ui_scale_percent: u8,
    font_size_percent: u8,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            ui_scale_percent: 100,
            font_size_percent: 100,
        }
    }
}

impl UiSettings {
    pub(crate) fn load() -> Self {
        match fs::read(settings_path()) {
            Ok(data) => match serde_json::from_slice::<Self>(&data) {
                Ok(mut settings) => {
                    settings.normalize();
                    settings
                }
                Err(error) => {
                    warn!("Could not read interface settings; using defaults: {error}");
                    Self::default()
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                warn!("Could not load interface settings; using defaults: {error}");
                Self::default()
            }
        }
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        let data = serde_json::to_vec_pretty(self).map_err(|error| {
            io::Error::other(format!("could not serialize interface settings: {error}"))
        })?;
        let mut file = AtomicWriteFile::open(settings_path())?;
        file.write_all(&data)?;
        file.commit()
    }

    pub(crate) fn ui_scale_percent(self) -> u8 {
        self.ui_scale_percent
    }

    pub(crate) fn font_size_percent(self) -> u8 {
        self.font_size_percent
    }

    fn ui_scale(self) -> f32 {
        f32::from(self.ui_scale_percent) / 100.0
    }

    fn font_scale(self) -> f32 {
        f32::from(self.font_size_percent) / 100.0
    }

    pub(crate) fn adjust_ui_scale(&mut self, delta: i16) -> bool {
        let next = (i16::from(self.ui_scale_percent) + delta).clamp(
            i16::from(MIN_UI_SCALE_PERCENT),
            i16::from(MAX_UI_SCALE_PERCENT),
        ) as u8;
        if next == self.ui_scale_percent {
            return false;
        }
        self.ui_scale_percent = next;
        true
    }

    pub(crate) fn adjust_font_size(&mut self, delta: i16) -> bool {
        let next = (i16::from(self.font_size_percent) + delta).clamp(
            i16::from(MIN_FONT_SIZE_PERCENT),
            i16::from(MAX_FONT_SIZE_PERCENT),
        ) as u8;
        if next == self.font_size_percent {
            return false;
        }
        self.font_size_percent = next;
        true
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    fn normalize(&mut self) {
        self.ui_scale_percent = self
            .ui_scale_percent
            .clamp(MIN_UI_SCALE_PERCENT, MAX_UI_SCALE_PERCENT);
        self.font_size_percent = self
            .font_size_percent
            .clamp(MIN_FONT_SIZE_PERCENT, MAX_FONT_SIZE_PERCENT);
    }
}

fn settings_path() -> std::path::PathBuf {
    crate::game_paths::GamePaths::get()
        .config_dir
        .join(SETTINGS_FILE_NAME)
}

pub(crate) fn install(app: &mut App) {
    app.insert_resource(UiSettings::load())
        .add_systems(Update, apply_ui_settings);
}

fn apply_ui_settings(
    settings: Res<UiSettings>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
    mut rem_size: ResMut<RemSize>,
    mut em_sizes: Query<&mut EmSize, Without<TextFont>>,
) {
    let target_ui_scale = settings.ui_scale();
    if (ui_scale.0 - target_ui_scale).abs() > f32::EPSILON {
        ui_scale.0 = target_ui_scale;
    }

    let viewport_min = window.width().min(window.height());
    let base_viewport_em = viewport_min * BASE_REM_VMIN_PERCENT;
    let target_rem_size = base_viewport_em * settings.font_scale();
    if (rem_size.0 - target_rem_size).abs() > 0.01 {
        rem_size.0 = target_rem_size;
    }

    let target_em_size = base_viewport_em * settings.ui_scale();
    for mut em_size in &mut em_sizes {
        if (em_size.0 - target_em_size).abs() > 0.01 {
            em_size.0 = target_em_size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_interface_preferences_are_neutral() {
        let settings = UiSettings::default();
        assert_eq!(settings.ui_scale_percent(), 100);
        assert_eq!(settings.font_size_percent(), 100);
        assert_eq!(settings.ui_scale(), 1.0);
        assert_eq!(settings.font_scale(), 1.0);
    }

    #[test]
    fn interface_preference_adjustments_respect_limits() {
        let mut settings = UiSettings::default();
        for _ in 0..20 {
            settings.adjust_ui_scale(SETTING_STEP);
            settings.adjust_font_size(-SETTING_STEP);
        }
        assert_eq!(settings.ui_scale_percent(), MAX_UI_SCALE_PERCENT);
        assert_eq!(settings.font_size_percent(), MIN_FONT_SIZE_PERCENT);
        assert!(!settings.adjust_ui_scale(SETTING_STEP));
        assert!(!settings.adjust_font_size(-SETTING_STEP));
    }

    #[test]
    fn reset_restores_both_interface_preferences() {
        let mut settings = UiSettings {
            ui_scale_percent: 120,
            font_size_percent: 145,
        };
        settings.reset();
        assert_eq!(settings, UiSettings::default());
    }

    #[test]
    fn loaded_preferences_are_clamped_to_supported_ranges() {
        let mut settings = UiSettings {
            ui_scale_percent: 255,
            font_size_percent: 0,
        };
        settings.normalize();
        assert_eq!(settings.ui_scale_percent(), MAX_UI_SCALE_PERCENT);
        assert_eq!(settings.font_size_percent(), MIN_FONT_SIZE_PERCENT);
    }
}
