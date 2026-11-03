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
    mut touch_targets: Query<(&mut Node, &crate::bevy_presentation::UiTouchTargetShape)>,
) {
    let target_ui_scale = settings.ui_scale();
    if (ui_scale.0 - target_ui_scale).abs() > f32::EPSILON {
        ui_scale.0 = target_ui_scale;
    }

    let viewport_min = window.width().min(window.height());
    if viewport_min > 0.0 {
        let base_viewport_em = viewport_min * BASE_REM_VMIN_PERCENT;
        let target_rem_size = base_viewport_em * settings.font_scale();
        if (rem_size.0 - target_rem_size).abs() > 0.01 {
            rem_size.0 = target_rem_size;
        }

        // Newly spawned UI nodes need their EmSize updated too, not only nodes
        // that existed when the user last changed a preference.
        let target_em_size = base_viewport_em * target_ui_scale;
        for mut em_size in &mut em_sizes {
            if (em_size.0 - target_em_size).abs() > 0.01 {
                em_size.0 = target_em_size;
            }
        }
    }

    // Bevy's UiScale also scales fixed-pixel dimensions. Compensate marked
    // interactive controls on Android so their final hit area remains at least
    // 48 logical pixels even when visual UI scaling is below 100%.
    let touch_scale = if cfg!(target_os = "android") {
        target_ui_scale
    } else {
        1.0
    };
    let target_touch_size = Val::Px(touch_target_base_size(
        crate::bevy_presentation::UI_TOUCH_TARGET_PX,
        touch_scale,
    ));
    for (mut node, shape) in &mut touch_targets {
        match shape {
            crate::bevy_presentation::UiTouchTargetShape::MinimumHeight => {
                if node.min_height != target_touch_size {
                    node.min_height = target_touch_size;
                }
            }
            crate::bevy_presentation::UiTouchTargetShape::MinimumBounds { width, height } => {
                let target_width = Val::Px(touch_target_base_size(f32::from(*width), touch_scale));
                let target_height =
                    Val::Px(touch_target_base_size(f32::from(*height), touch_scale));
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

    #[test]
    fn touch_target_compensation_preserves_48_logical_pixels() {
        for scale in [0.75, 1.0, 1.25] {
            let base_size =
                touch_target_base_size(crate::bevy_presentation::UI_TOUCH_TARGET_PX, scale);
            assert!(
                (base_size * scale - crate::bevy_presentation::UI_TOUCH_TARGET_PX).abs() < 0.001
            );
        }
    }

    #[test]
    fn applying_settings_updates_layout_typography_and_touch_targets() {
        let mut world = World::new();
        let settings = UiSettings {
            ui_scale_percent: 75,
            font_size_percent: 150,
        };
        world.insert_resource(settings);
        world.insert_resource(UiScale(1.0));
        world.insert_resource(RemSize(20.0));
        let window = world
            .spawn((
                Window {
                    resolution: (1280, 720).into(),
                    ..default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();

        let layout_node = world.spawn((Node::default(), EmSize(20.0))).id();
        let text_node = world
            .spawn((Node::default(), EmSize(20.0), TextFont::default()))
            .id();
        let height_target = world
            .spawn((
                Node {
                    min_height: Val::Px(crate::bevy_presentation::UI_TOUCH_TARGET_PX),
                    ..default()
                },
                crate::bevy_presentation::UiTouchTargetShape::MinimumHeight,
            ))
            .id();
        let minimum_bounds_target = world
            .spawn((
                Node {
                    min_width: Val::Px(52.0),
                    min_height: Val::Px(crate::bevy_presentation::UI_TOUCH_TARGET_PX),
                    ..default()
                },
                crate::bevy_presentation::UiTouchTargetShape::MinimumBounds {
                    width: 52,
                    height: 48,
                },
            ))
            .id();
        let square_target = world
            .spawn((
                Node {
                    width: Val::Px(crate::bevy_presentation::UI_TOUCH_TARGET_PX),
                    min_width: Val::Px(crate::bevy_presentation::UI_TOUCH_TARGET_PX),
                    height: Val::Px(crate::bevy_presentation::UI_TOUCH_TARGET_PX),
                    min_height: Val::Px(crate::bevy_presentation::UI_TOUCH_TARGET_PX),
                    ..default()
                },
                crate::bevy_presentation::UiTouchTargetShape::Square,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(apply_ui_settings);
        schedule.run(&mut world);

        let base_viewport_em = 720.0 * BASE_REM_VMIN_PERCENT;
        assert!((world.resource::<UiScale>().0 - 0.75).abs() < f32::EPSILON);
        assert!((world.resource::<RemSize>().0 - base_viewport_em * 1.5).abs() < 0.01);
        assert!(
            (world.get::<EmSize>(layout_node).unwrap().0 - base_viewport_em * 0.75).abs() < 0.01
        );
        assert!((world.get::<EmSize>(text_node).unwrap().0 - 20.0).abs() < 0.01);

        let touch_scale = if cfg!(target_os = "android") {
            0.75
        } else {
            1.0
        };
        let expected_touch_size =
            touch_target_base_size(crate::bevy_presentation::UI_TOUCH_TARGET_PX, touch_scale);
        assert_eq!(
            world.get::<Node>(height_target).unwrap().min_height,
            Val::Px(expected_touch_size)
        );

        let bounds = world.get::<Node>(minimum_bounds_target).unwrap();
        assert_eq!(
            bounds.min_width,
            Val::Px(touch_target_base_size(52.0, touch_scale))
        );
        assert_eq!(bounds.min_height, Val::Px(expected_touch_size));

        let square = world.get::<Node>(square_target).unwrap();
        assert_eq!(square.width, Val::Px(expected_touch_size));
        assert_eq!(square.min_width, Val::Px(expected_touch_size));
        assert_eq!(square.height, Val::Px(expected_touch_size));
        assert_eq!(square.min_height, Val::Px(expected_touch_size));

        world
            .resource_mut::<UiSettings>()
            .adjust_ui_scale(SETTING_STEP);
        world
            .resource_mut::<UiSettings>()
            .adjust_font_size(-SETTING_STEP);
        let late_layout_node = world.spawn((Node::default(), EmSize(1.0))).id();
        world.get_mut::<Window>(window).unwrap().resolution = (1920, 1080).into();
        schedule.run(&mut world);

        let resized_viewport_em = 1080.0 * BASE_REM_VMIN_PERCENT;
        assert!((world.resource::<UiScale>().0 - 0.80).abs() < f32::EPSILON);
        assert!((world.resource::<RemSize>().0 - resized_viewport_em * 1.45).abs() < 0.01);
        assert!(
            (world.get::<EmSize>(layout_node).unwrap().0 - resized_viewport_em * 0.80).abs()
                < 0.01
        );
        assert!(
            (world.get::<EmSize>(late_layout_node).unwrap().0 - resized_viewport_em * 0.80).abs()
                < 0.01
        );
        assert!((world.get::<EmSize>(text_node).unwrap().0 - 20.0).abs() < 0.01);

        let resized_touch_scale = if cfg!(target_os = "android") {
            0.80
        } else {
            1.0
        };
        let resized_touch_size = touch_target_base_size(
            crate::bevy_presentation::UI_TOUCH_TARGET_PX,
            resized_touch_scale,
        );
        assert_eq!(
            world.get::<Node>(height_target).unwrap().min_height,
            Val::Px(resized_touch_size)
        );

        let bounds = world.get::<Node>(minimum_bounds_target).unwrap();
        assert_eq!(
            bounds.min_width,
            Val::Px(touch_target_base_size(52.0, resized_touch_scale))
        );
        assert_eq!(bounds.min_height, Val::Px(resized_touch_size));

        let square = world.get::<Node>(square_target).unwrap();
        assert_eq!(square.width, Val::Px(resized_touch_size));
        assert_eq!(square.min_width, Val::Px(resized_touch_size));
        assert_eq!(square.height, Val::Px(resized_touch_size));
        assert_eq!(square.min_height, Val::Px(resized_touch_size));
    }
}
