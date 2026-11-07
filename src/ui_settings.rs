use std::fs;
use std::io::{self, Write};

use atomic_write_file::AtomicWriteFile;
use bevy::prelude::*;
use bevy::ui::prelude::{EmSize, RemSize};
use bevy::ui::UiScale;
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
        let next = adjusted_percent(
            self.ui_scale_percent,
            delta,
            MIN_UI_SCALE_PERCENT,
            MAX_UI_SCALE_PERCENT,
        );
        if next == self.ui_scale_percent {
            return false;
        }
        self.ui_scale_percent = next;
        true
    }

    pub(crate) fn adjust_font_size(&mut self, delta: i16) -> bool {
        let next = adjusted_percent(
            self.font_size_percent,
            delta,
            MIN_FONT_SIZE_PERCENT,
            MAX_FONT_SIZE_PERCENT,
        );
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

fn adjusted_percent(current: u8, delta: i16, minimum: u8, maximum: u8) -> u8 {
    i16::from(current)
        .saturating_add(delta)
        .clamp(i16::from(minimum), i16::from(maximum)) as u8
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
    mut em_sizes: ParamSet<(
        Query<&mut EmSize, Without<TextFont>>,
        Query<&mut EmSize, (Without<TextFont>, Added<EmSize>)>,
    )>,
    mut touch_targets: ParamSet<(
        Query<(&mut Node, &crate::bevy_presentation::UiTouchTargetShape)>,
        Query<
            (&mut Node, &crate::bevy_presentation::UiTouchTargetShape),
            Added<crate::bevy_presentation::UiTouchTargetShape>,
        >,
    )>,
    mut last_settings: Local<Option<UiSettings>>,
    mut last_viewport_min: Local<f32>,
) {
    let target_ui_scale = settings.ui_scale();
    if (ui_scale.0 - target_ui_scale).abs() > f32::EPSILON {
        ui_scale.0 = target_ui_scale;
    }

    let viewport_min = window.width().min(window.height());
    let apply_all =
        *last_settings != Some(*settings) || (*last_viewport_min - viewport_min).abs() > 0.01;
    let base_viewport_em = viewport_min * BASE_REM_VMIN_PERCENT;
    let target_em_size = base_viewport_em * target_ui_scale;
    let touch_scale = if cfg!(target_os = "android") {
        target_ui_scale
    } else {
        1.0
    };

    if apply_all {
        if viewport_min > 0.0 {
            let target_rem_size = base_viewport_em * settings.font_scale();
            if (rem_size.0 - target_rem_size).abs() > 0.01 {
                rem_size.0 = target_rem_size;
            }

            let mut em_query = em_sizes.p0();
            for mut em_size in em_query.iter_mut() {
                if (em_size.0 - target_em_size).abs() > 0.01 {
                    em_size.0 = target_em_size;
                }
            }
        }

        let mut touch_query = touch_targets.p0();
        for (mut node, shape) in touch_query.iter_mut() {
            apply_touch_target_shape(&mut node, shape, touch_scale);
        }

        *last_settings = Some(*settings);
        *last_viewport_min = viewport_min;
    } else {
        // New UI nodes still need their EmSize and touch-target compensation,
        // but existing nodes do not need to be scanned every frame.
        if viewport_min > 0.0 {
            let mut em_query = em_sizes.p1();
            for mut em_size in em_query.iter_mut() {
                if (em_size.0 - target_em_size).abs() > 0.01 {
                    em_size.0 = target_em_size;
                }
            }
        }

        let mut touch_query = touch_targets.p1();
        for (mut node, shape) in touch_query.iter_mut() {
            apply_touch_target_shape(&mut node, shape, touch_scale);
        }
    }
}

fn apply_touch_target_shape(
    node: &mut Node,
    shape: &crate::bevy_presentation::UiTouchTargetShape,
    touch_scale: f32,
) {
    let target_touch_size = Val::Px(touch_target_base_size(
        crate::bevy_presentation::UI_TOUCH_TARGET_PX,
        touch_scale,
    ));
    match shape {
        crate::bevy_presentation::UiTouchTargetShape::MinimumHeight => {
            if node.min_height != target_touch_size {
                node.min_height = target_touch_size;
            }
        }
        crate::bevy_presentation::UiTouchTargetShape::MinimumBounds { width, height } => {
            let target_width = Val::Px(touch_target_base_size(f32::from(*width), touch_scale));
            let target_height = Val::Px(touch_target_base_size(f32::from(*height), touch_scale));
            if node.min_width != target_width {
                node.min_width = target_width;
            }
            if node.min_height != target_height {
                node.min_height = target_height;
            }
        }
        crate::bevy_presentation::UiTouchTargetShape::Square => {
            if node.width != target_touch_size {
                node.width = target_touch_size;
            }
            if node.min_width != target_touch_size {
                node.min_width = target_touch_size;
            }
            if node.height != target_touch_size {
                node.height = target_touch_size;
            }
            if node.min_height != target_touch_size {
                node.min_height = target_touch_size;
            }
        }
    }
}

fn touch_target_base_size(target_size: f32, ui_scale: f32) -> f32 {
    let scale = if ui_scale.is_finite() {
        ui_scale.clamp(
            f32::from(MIN_UI_SCALE_PERCENT) / 100.0,
            f32::from(MAX_UI_SCALE_PERCENT) / 100.0,
        )
    } else {
        1.0
    };
    target_size / scale
}

#[cfg(test)]
#[path = "../tests/unit/ui_settings.rs"]
mod tests;
