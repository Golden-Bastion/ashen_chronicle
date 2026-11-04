//! Reusable Bevy presentation primitives for the graphical frontend.
//!
//! This module depends only on Bevy and the frontend-neutral semantic input
//! model. Gameplay systems remain responsible for translating authoritative game
//! state into presentation view models and interpreting semantic input events.

use bevy::input::keyboard::{Key, KeyboardInput, NativeKeyCode};
use bevy::input::mouse::MouseWheel;
use bevy::input::ButtonState;
use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex},
    AutoFocus, FocusCause, InputFocus,
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{EditableText, Justify, LineBreak, TextCursorStyle, TextLayout};
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::PrimaryWindow;
use std::collections::{HashMap, HashSet};
#[cfg(target_os = "android")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use crate::input::InputEvent;

pub const THEME_BACKGROUND: Color = Color::srgb(0.055, 0.045, 0.065);
pub const THEME_PANEL: Color = Color::srgb(0.105, 0.085, 0.12);
pub const THEME_PANEL_ALT: Color = Color::srgb(0.135, 0.11, 0.15);
pub const THEME_TEXT: Color = Color::srgb(0.9, 0.88, 0.82);
pub const THEME_MUTED: Color = Color::srgb(0.62, 0.6, 0.58);
pub const THEME_ACCENT: Color = Color::srgb(0.72, 0.62, 0.46);
pub const THEME_SURFACE: Color = Color::srgba(0.07, 0.06, 0.08, 0.78);
pub const THEME_SURFACE_STRONG: Color = Color::srgba(0.105, 0.085, 0.12, 0.94);
pub const THEME_OVERLAY: Color = Color::srgba(0.02, 0.018, 0.025, 0.82);
pub const THEME_OVERLAY_SOFT: Color = Color::srgba(0.02, 0.018, 0.025, 0.58);
pub const THEME_BLOOD: Color = Color::srgb(0.42, 0.035, 0.025);
pub const THEME_BLOOD_DARK: Color = Color::srgb(0.11, 0.012, 0.012);
pub const THEME_BORDER: Color = Color::srgba(0.35, 0.30, 0.24, 0.72);
pub const THEME_HOVER: Color = Color::srgba(0.20, 0.16, 0.12, 0.92);
pub const THEME_PRESSED: Color = Color::srgba(0.30, 0.24, 0.17, 0.96);
pub const THEME_SELECTED: Color = Color::srgba(0.16, 0.13, 0.10, 0.90);
pub const THEME_SKY_DAY: Color = Color::srgb(0.19, 0.25, 0.32);
pub const THEME_SKY_DAWN: Color = Color::srgb(0.38, 0.19, 0.17);
pub const THEME_SKY_DUSK: Color = Color::srgb(0.32, 0.13, 0.14);
pub const THEME_SKY_NIGHT: Color = Color::srgb(0.08, 0.045, 0.085);
pub const THEME_SKY_HORIZON_WARM: Color = Color::srgba(0.48, 0.15, 0.075, 0.56);

pub const UI_TOUCH_TARGET_PX: f32 = 48.0;
pub const UI_COMPACT_GAP_VMIN: f32 = 1.111;
pub const UI_SURFACE_PADDING_VMIN: f32 = 1.667;
pub const UI_ACTION_PADDING_HORIZONTAL_VMIN: f32 = 1.944;
pub const UI_ACTION_PADDING_VERTICAL_VMIN: f32 = 0.833;
pub const UI_ACTION_MAX_WIDTH_PX: f32 = 520.0;

#[derive(Component)]
pub struct UiSurface;

#[derive(Component)]
pub struct UiOverlayRoot;

#[derive(Component)]
pub struct UiActionButton;

#[derive(Component)]
struct UiActionButtonGroup;

#[derive(Component)]
struct UiStackedActionButton;

#[derive(Component)]
pub struct UiMenuButton;

#[derive(Component)]
pub struct UiConditionIndicator;

#[derive(Component)]
pub struct UiContextMessage;

#[derive(Component)]
pub struct UiStyledButton;

#[derive(Component)]
pub struct UiSelected;

/// Marks interactive nodes whose minimum hit area must remain 48 logical pixels
/// on Android, independent of the user's visual UI scale.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiTouchTargetShape {
    MinimumHeight,
    MinimumBounds { width: u16, height: u16 },
    Square,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UiTabButton {
    pub(crate) index: usize,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiHealthGauge {
    pub current: i32,
    pub maximum: i32,
}

#[derive(Component)]
struct UiHealthGaugeFill;

#[derive(Component)]
struct UiHealthGaugeShadow;

#[derive(Component)]
struct UiGaugeValueText;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceTone {
    #[allow(dead_code)] // TODO: this varient is never constructed; remove thid after it is used
    Quiet,
    Strong,
}

#[derive(Component)]
pub struct BevyScreenRoot;

#[derive(Component)]
pub struct ChoiceButton {
    pub index: usize,
}

#[derive(Component)]
pub struct TextContent;

#[derive(Component)]
pub struct GaugeFill;

#[derive(Component)]
pub(crate) struct TouchScrollablePanel;

#[derive(Component)]
pub(crate) struct LifecycleTextField {
    pub(crate) index: usize,
}

#[derive(Component)]
struct ContextualConsoleInput;

#[derive(Component)]
struct ContextualConsoleTab;

#[derive(Resource, Default, Debug)]
pub(crate) struct ConsoleTextInputFocus {
    pub(crate) active: bool,
    suppress_next_back: bool,
}

#[cfg(target_os = "android")]
static ANDROID_BACK_REQUESTED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "android")]
static ANDROID_TEXT_INPUT_EVENTS: Mutex<Vec<(u8, AndroidTextInputEvent)>> = Mutex::new(Vec::new());

#[cfg(target_os = "android")]
#[derive(Debug)]
pub(crate) enum AndroidTextInputEvent {
    Changed(String),
    Submitted,
    Dismissed,
}

#[cfg(target_os = "android")]
fn queue_android_text_input_event(target: i32, event: AndroidTextInputEvent) {
    let Ok(target) = u8::try_from(target) else {
        return;
    };
    if target > 1 {
        return;
    }

    if let Ok(mut events) = ANDROID_TEXT_INPUT_EVENTS.lock() {
        events.push((target, event));
    }
}

#[cfg(target_os = "android")]
pub(crate) fn android_take_text_input_events(target: u8) -> Vec<AndroidTextInputEvent> {
    let Ok(mut events) = ANDROID_TEXT_INPUT_EVENTS.lock() else {
        return Vec::new();
    };

    let mut selected = Vec::new();
    let mut retained = Vec::with_capacity(events.len());
    for (event_target, event) in events.drain(..) {
        if event_target == target {
            selected.push(event);
        } else {
            retained.push((event_target, event));
        }
    }
    *events = retained;
    selected
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rightward_ashenchronicle_MainActivity_nativeBackNavigation(
    _env: jni::EnvUnowned<'_>,
    _class: jni::objects::JClass<'_>,
) {
    ANDROID_BACK_REQUESTED.store(true, Ordering::Release);
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rightward_ashenchronicle_MainActivity_nativeAndroidTextInputChanged(
    mut env: jni::EnvUnowned<'_>,
    _class: jni::objects::JClass<'_>,
    target: i32,
    text: jni::objects::JString<'_>,
) {
    match env
        .with_env(|env| -> jni::errors::Result<()> {
            let text = text.try_to_string(env)?;
            queue_android_text_input_event(target, AndroidTextInputEvent::Changed(text));
            Ok(())
        })
        .into_outcome()
    {
        jni::Outcome::Ok(()) => {}
        jni::Outcome::Err(error) => {
            bevy::log::warn!("Could not decode Android text input: {error}");
        }
        jni::Outcome::Panic(_) => {
            bevy::log::warn!("Android text input JNI callback panicked");
        }
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rightward_ashenchronicle_MainActivity_nativeAndroidTextInputSubmitted(
    _env: jni::EnvUnowned<'_>,
    _class: jni::objects::JClass<'_>,
    target: i32,
) {
    queue_android_text_input_event(target, AndroidTextInputEvent::Submitted);
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rightward_ashenchronicle_MainActivity_nativeAndroidTextInputDismissed(
    _env: jni::EnvUnowned<'_>,
    _class: jni::objects::JClass<'_>,
    target: i32,
) {
    queue_android_text_input_event(target, AndroidTextInputEvent::Dismissed);
}

const TOUCH_SCROLL_THRESHOLD_PX: f32 = 12.0;

fn touch_scroll_dragged(start: Vec2, current: Vec2) -> bool {
    current.distance(start) > TOUCH_SCROLL_THRESHOLD_PX
}

#[derive(Clone, Copy, Debug)]
struct ActiveTouchScroll {
    id: u64,
    panel: Entity,
    start: Vec2,
    last: Vec2,
    button: Option<Entity>,
    dragging: bool,
}

#[derive(Resource, Default, Debug)]
struct TouchScrollState {
    active: Option<ActiveTouchScroll>,
}

impl TouchScrollState {
    fn should_suppress_activation(&self, button: Entity, touches: &Touches) -> bool {
        let Some(active) = self.active else {
            return false;
        };
        if active.button != Some(button) {
            return false;
        }
        if active.dragging {
            return true;
        }

        let current_position = touches
            .iter()
            .find(|touch| touch.id() == active.id)
            .map(|touch| touch.position())
            .or_else(|| {
                touches
                    .iter_just_released()
                    .find(|touch| touch.id() == active.id)
                    .map(|touch| touch.position())
            });
        current_position.is_some_and(|position| touch_scroll_dragged(active.start, position))
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct UiButtonActivation(pub(crate) Entity);

#[derive(Resource, Default)]
pub struct SemanticInputQueue(pub Vec<InputEvent>);

#[derive(Resource, Default)]
pub struct GameplayInputQueue(pub Vec<InputEvent>);

pub(crate) fn physical_touch_position(position: Vec2, scale_factor: f32) -> Vec2 {
    position * scale_factor
}

#[cfg(target_os = "android")]
pub(crate) fn android_set_text_input_target(target: Option<u8>, text: &str) {
    if let Ok(mut events) = ANDROID_TEXT_INPUT_EVENTS.lock() {
        events.clear();
    }

    let Some(app) = bevy::android::ANDROID_APP.get().cloned() else {
        bevy::log::warn!(
            "Could not set Android text input target: Android app handle is unavailable"
        );
        return;
    };

    let java_target = target.map(i32::from).unwrap_or(-1);
    let text = text.to_owned();
    let java_app = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(java_app.vm_as_ptr().cast()) };
        if let Err(error) = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let raw_activity = java_app.activity_as_ptr() as jni::sys::jobject;
            let activity = unsafe {
                env.as_cast_raw::<jni::refs::Global<jni::objects::JObject>>(&raw_activity)?
            };
            let value = env.new_string(&text)?;
            env.call_method(
                activity.as_ref(),
                jni::jni_str!("setAndroidTextInput"),
                jni::jni_sig!("(ILjava/lang/String;)V"),
                &[jni::JValue::from(java_target), jni::JValue::from(&value)],
            )?;
            Ok(())
        }) {
            bevy::log::warn!("Could not update Android native text input: {error}");
        }
    }));
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationState {
    pub current_screen: Option<ScreenId>,
    pub return_screen: Option<ScreenId>,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    Lifecycle,
    Gameplay,
    Character,
    Inventory,
    Quests,
    Meditation,
    History,
    Journal,
    Talk,
    Conversation,
    Remains,
    RemainsResult,
    SpatialExploration,
    Combat,
    Console,
    Options,
    Feedback,
}

fn screen_padding() -> Val {
    responsive_vmin(3.333)
}

fn screen_gap() -> Val {
    responsive_vmin(2.222)
}

fn panel_padding() -> Val {
    responsive_vmin(2.222)
}

fn panel_gap() -> Val {
    responsive_vmin(1.389)
}

/// Converts a viewport-minimum percentage into an Em value. Non-text UI nodes
/// receive a shared EmSize from ui_settings, allowing the user UI scale to
/// scale responsive layout spacing without coupling it to the font-size setting.
pub(crate) fn responsive_vmin(percent: f32) -> Val {
    em(percent / 2.778)
}

fn title_font_size() -> FontSize {
    FontSize::Rem(1.8)
}

fn label_font_size() -> FontSize {
    FontSize::Rem(1.0)
}

pub(crate) fn muted_font_size() -> FontSize {
    FontSize::Rem(0.9)
}

pub fn install(app: &mut App) {
    app.add_message::<UiButtonActivation>()
        .add_observer(queue_button_activation)
        .init_resource::<SemanticInputQueue>()
        .init_resource::<GameplayInputQueue>()
        .init_resource::<NavigationState>()
        .init_resource::<ConsoleTextInputFocus>()
        .init_resource::<TouchScrollState>()
        .init_resource::<UiIconAssets>()
        .add_systems(
            PreUpdate,
            process_ime_events
                .after(bevy::input::InputSystems)
                .after(bevy::ui_widgets::ImeSystems::HandleEvents)
                .after(bevy::ui_widgets::ImeSystems::ToggleWindowIMEInput),
        )
        .add_systems(Update, keyboard_to_semantic_input)
        .add_systems(Update, mouse_to_semantic_input)
        .add_systems(Update, choice_button_input)
        .add_systems(Update, contextual_touch_input)
        .add_systems(PostUpdate, sync_ui_health_gauges)
        .add_systems(PostUpdate, sync_ui_button_visuals)
        .add_systems(Update, touch_scroll)
        .add_systems(PostUpdate, resolve_ui_icons)
        .add_systems(PostUpdate, organize_action_button_groups)
        .add_systems(PostUpdate, sync_ime_window)
        .add_systems(PostUpdate, sync_lifecycle_field_visuals);

    #[cfg(target_os = "android")]
    app.add_systems(Update, android_back_to_semantic_input);
}

pub fn spawn_world_root(commands: &mut Commands, background: Color) -> Entity {
    commands
        .spawn((
            BevyScreenRoot,
            Node {
                width: percent(100),
                height: percent(100),
                min_width: px(0),
                min_height: px(0),
                padding: UiRect::all(screen_padding()),
                flex_direction: FlexDirection::Column,
                row_gap: responsive_compact_gap(),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(background),
        ))
        .id()
}

pub fn spawn_screen(commands: &mut Commands, title: impl Into<String>) -> Entity {
    commands
        .spawn((
            BevyScreenRoot,
            Node {
                width: percent(100),
                height: percent(100),
                min_width: px(0),
                min_height: px(0),
                padding: UiRect::all(screen_padding()),
                flex_direction: FlexDirection::Column,
                row_gap: screen_gap(),
                ..default()
            },
            BackgroundColor(THEME_BACKGROUND),
            children![(
                Text::new(title.into()),
                TextContent,
                TextFont::from_font_size(title_font_size()),
                TextColor(THEME_TEXT),
            )],
        ))
        .id()
}

pub fn spawn_panel(commands: &mut Commands, parent: Entity) -> Entity {
    let panel = commands
        .spawn((
            TouchScrollablePanel,
            TabGroup::modal(),
            Node {
                width: percent(100),
                min_width: px(0),
                min_height: px(0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                padding: UiRect::all(panel_padding()),
                flex_direction: FlexDirection::Column,
                row_gap: panel_gap(),
                overflow: Overflow::scroll(),
                ..default()
            },
            BackgroundColor(THEME_PANEL),
        ))
        .id();
    commands.entity(parent).add_child(panel);
    panel
}

fn fixed_panel_node() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        min_height: px(0),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        padding: UiRect::all(panel_padding()),
        flex_direction: FlexDirection::Column,
        row_gap: panel_gap(),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// Creates a fixed screen panel; only its dedicated list child should scroll.
pub fn spawn_fixed_panel(commands: &mut Commands, parent: Entity) -> Entity {
    let panel = commands
        .spawn((
            TabGroup::modal(),
            fixed_panel_node(),
            BackgroundColor(THEME_PANEL),
        ))
        .id();
    commands.entity(parent).add_child(panel);
    panel
}

fn scrollable_list_node() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        min_height: px(0),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_direction: FlexDirection::Column,
        row_gap: panel_gap(),
        overflow: Overflow::scroll(),
        ..default()
    }
}

/// Creates the only scrolling region inside a fixed Options-style panel.
pub fn spawn_scrollable_list(commands: &mut Commands, parent: Entity) -> Entity {
    let list = commands
        .spawn((TouchScrollablePanel, scrollable_list_node()))
        .id();
    commands.entity(parent).add_child(list);
    list
}

fn option_footer_node() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        min_height: touch_target_size(),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: responsive_compact_gap(),
        row_gap: responsive_compact_gap(),
        justify_content: JustifyContent::End,
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        padding: UiRect::top(responsive_compact_gap()),
        ..default()
    }
}

/// Creates a non-scrolling footer below the option list.
pub fn spawn_option_footer(commands: &mut Commands, parent: Entity) -> Entity {
    let footer = commands.spawn(option_footer_node()).id();
    commands.entity(parent).add_child(footer);
    footer
}

fn standalone_label_node() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        max_width: percent(100),
        flex_shrink: 0.0,
        ..default()
    }
}

pub fn spawn_label(commands: &mut Commands, parent: Entity, text: impl Into<String>) -> Entity {
    let label = commands
        .spawn((
            Text::new(text.into()),
            standalone_label_node(),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            TextContent,
            TextFont::from_font_size(label_font_size()),
            TextColor(THEME_TEXT),
        ))
        .id();
    commands.entity(parent).add_child(label);
    label
}

pub fn spawn_muted_label(
    commands: &mut Commands,
    parent: Entity,
    text: impl Into<String>,
) -> Entity {
    let label = commands
        .spawn((
            Text::new(text.into()),
            standalone_label_node(),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            TextContent,
            TextFont::from_font_size(muted_font_size()),
            TextColor(THEME_MUTED),
        ))
        .id();
    commands.entity(parent).add_child(label);
    label
}

pub fn spawn_text_input_field(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    label: impl Into<String>,
    value: impl Into<String>,
    selected: bool,
) -> Entity {
    let label = label.into();
    let value = value.into();
    let row = commands
        .spawn(Node {
            width: percent(100),
            min_width: px(0),
            min_height: px(48),
            flex_direction: FlexDirection::Row,
            column_gap: responsive_vmin(1.389),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();

    let label_entity = commands
        .spawn((
            Text::new(format!("{label}:")),
            Node {
                width: percent(25),
                min_width: px(0),
                ..default()
            },
            TextContent,
            TextFont::from_font_size(label_font_size()),
            TextColor(THEME_TEXT),
        ))
        .id();

    let field = commands
        .spawn((
            Button,
            Hovered::default(),
            UiTouchTargetShape::MinimumHeight,
            LifecycleTextField { index },
            EditableText::new(&value),
            TextCursorStyle::default(),
            TextLayout::no_wrap(),
            TabIndex(index as i32),
            Node {
                width: percent(75),
                min_width: px(0),
                min_height: px(48),
                padding: UiRect::axes(responsive_vmin(1.389), responsive_vmin(0.833)),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(if selected { THEME_ACCENT } else { THEME_MUTED }),
            BackgroundColor(THEME_PANEL_ALT),
            TextFont::from_font_size(label_font_size()),
            TextColor(THEME_TEXT),
        ))
        .id();

    if selected {
        commands.entity(field).insert(AutoFocus);
    }

    commands.entity(row).add_child(label_entity);
    commands.entity(row).add_child(field);
    commands.entity(parent).add_child(row);
    field
}

pub(crate) fn spawn_console_input_row(
    commands: &mut Commands,
    parent: Entity,
    text: impl Into<String>,
) -> Entity {
    let row = commands
        .spawn(Node {
            width: percent(100),
            min_width: px(0),
            flex_direction: FlexDirection::Row,
            column_gap: responsive_vmin(1.111),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();

    let input = commands
        .spawn((
            Button,
            Hovered::default(),
            UiTouchTargetShape::MinimumHeight,
            ContextualConsoleInput,
            Text::new(text.into()),
            Node {
                width: percent(84),
                min_width: px(0),
                max_width: percent(100),
                min_height: px(48),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                padding: UiRect::axes(responsive_vmin(1.389), responsive_vmin(0.833)),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                ..default()
            },
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            TextContent,
            TextFont::from_font_size(label_font_size()),
            TextColor(THEME_TEXT),
            BorderColor::all(THEME_ACCENT),
            BackgroundColor(THEME_PANEL_ALT),
        ))
        .id();

    let tab = commands
        .spawn((
            Button,
            Hovered::default(),
            UiTouchTargetShape::MinimumHeight,
            ContextualConsoleTab,
            Node {
                width: px(64),
                min_width: px(64),
                min_height: px(48),
                padding: UiRect::axes(px(10), px(6)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(THEME_ACCENT),
            BackgroundColor(THEME_PANEL_ALT),
            children![(
                Text::new("Tab"),
                TextContent,
                TextFont::from_font_size(muted_font_size()),
                TextColor(THEME_TEXT),
            )],
        ))
        .id();

    commands.entity(row).add_child(input);
    commands.entity(row).add_child(tab);
    commands.entity(parent).add_child(row);
    input
}

pub fn responsive_compact_gap() -> Val {
    responsive_vmin(UI_COMPACT_GAP_VMIN)
}

pub fn responsive_surface_padding() -> Val {
    responsive_vmin(UI_SURFACE_PADDING_VMIN)
}

pub fn touch_target_size() -> Val {
    px(UI_TOUCH_TARGET_PX)
}

pub fn spawn_surface(commands: &mut Commands, parent: Entity, tone: SurfaceTone) -> Entity {
    let background = match tone {
        SurfaceTone::Quiet => THEME_SURFACE,
        SurfaceTone::Strong => THEME_SURFACE_STRONG,
    };
    let surface = commands
        .spawn((
            UiSurface,
            Node {
                width: percent(100),
                min_width: px(0),
                min_height: px(0),
                padding: UiRect::all(responsive_surface_padding()),
                flex_direction: FlexDirection::Column,
                row_gap: responsive_compact_gap(),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(THEME_BORDER),
            BackgroundColor(background),
        ))
        .id();
    commands.entity(parent).add_child(surface);
    surface
}

pub fn spawn_overlay(commands: &mut Commands, parent: Entity) -> Entity {
    let overlay = commands
        .spawn((
            UiOverlayRoot,
            TabGroup::modal(),
            Node {
                width: percent(100),
                height: percent(100),
                min_width: px(0),
                min_height: px(0),
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                padding: UiRect::all(screen_padding()),
                flex_direction: FlexDirection::Column,
                row_gap: responsive_compact_gap(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(THEME_OVERLAY_SOFT),
        ))
        .id();
    commands.entity(parent).add_child(overlay);
    overlay
}

fn action_button_group_node() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        max_width: px(UI_ACTION_MAX_WIDTH_PX),
        flex_direction: FlexDirection::Column,
        row_gap: responsive_compact_gap(),
        flex_shrink: 0.0,
        align_self: AlignSelf::FlexStart,
        margin: UiRect {
            top: Val::Auto,
            ..default()
        },
        ..default()
    }
}

fn action_button_node() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        max_width: percent(100),
        min_height: touch_target_size(),
        flex_grow: 0.0,
        flex_shrink: 0.0,
        padding: UiRect::axes(
            responsive_vmin(UI_ACTION_PADDING_HORIZONTAL_VMIN),
            responsive_vmin(UI_ACTION_PADDING_VERTICAL_VMIN),
        ),
        border: UiRect::all(px(1)),
        justify_content: JustifyContent::Start,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// The two content slots created by a shared Options/settings row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionRowSlots {
    pub row: Entity,
    pub label: Entity,
    pub controls: Entity,
}

#[derive(Component)]
struct OptionRowLabelSlot;

#[derive(Component)]
struct OptionRowControlsSlot;

fn option_row_node() -> Node {
    Node {
        width: percent(100),
        min_width: px(0),
        min_height: touch_target_size(),
        padding: UiRect::axes(responsive_surface_padding(), responsive_vmin(0.833)),
        border: UiRect::all(px(1)),
        flex_direction: FlexDirection::Row,
        column_gap: responsive_compact_gap(),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    }
}

fn option_row_label_node() -> Node {
    Node {
        min_width: px(0),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_direction: FlexDirection::Column,
        row_gap: px(2),
        align_items: AlignItems::Start,
        ..default()
    }
}

fn option_row_controls_node() -> Node {
    Node {
        min_width: px(0),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Row,
        column_gap: responsive_compact_gap(),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::End,
        ..default()
    }
}

fn compact_action_button_node() -> Node {
    Node {
        width: Val::Auto,
        min_width: px(76),
        max_width: px(180),
        min_height: touch_target_size(),
        flex_grow: 0.0,
        flex_shrink: 0.0,
        padding: UiRect::axes(
            responsive_vmin(1.111),
            responsive_vmin(UI_ACTION_PADDING_VERTICAL_VMIN),
        ),
        border: UiRect::all(px(1)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

fn option_value_node() -> Node {
    Node {
        min_width: px(58),
        min_height: touch_target_size(),
        flex_shrink: 0.0,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// Creates the shared landscape option row: the option name stays on the left
/// and its controls stay together on the right.
pub fn spawn_option_row(commands: &mut Commands, parent: Entity) -> OptionRowSlots {
    let row = commands
        .spawn((
            UiSurface,
            option_row_node(),
            BorderColor::all(THEME_BORDER),
            BackgroundColor(THEME_SURFACE),
        ))
        .id();
    let label = commands
        .spawn((OptionRowLabelSlot, option_row_label_node()))
        .id();
    let controls = commands
        .spawn((OptionRowControlsSlot, option_row_controls_node()))
        .id();

    commands.entity(row).add_child(label);
    commands.entity(row).add_child(controls);
    commands.entity(parent).add_child(row);

    OptionRowSlots {
        row,
        label,
        controls,
    }
}

/// Adds a current setting value to the right-hand control group.
pub fn spawn_option_value(
    commands: &mut Commands,
    parent: Entity,
    text: impl Into<String>,
) -> Entity {
    let value = commands
        .spawn((
            Text::new(text.into()),
            option_value_node(),
            TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
            TextContent,
            TextFont::from_font_size(muted_font_size()),
            TextColor(THEME_TEXT),
        ))
        .id();
    commands.entity(parent).add_child(value);
    value
}

/// Spawns a standard horizontal button for option rows and persistent footers.
pub fn spawn_compact_action_button(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    label: impl Into<String>,
) -> Entity {
    let button = commands
        .spawn((
            Button,
            Hovered::default(),
            UiTouchTargetShape::MinimumBounds {
                width: UI_TOUCH_TARGET_PX as u16,
                height: UI_TOUCH_TARGET_PX as u16,
            },
            ChoiceButton { index },
            UiActionButton,
            UiStyledButton,
            compact_action_button_node(),
            BorderColor::all(THEME_BORDER),
            BackgroundColor(THEME_SURFACE),
            children![(
                Text::new(label.into()),
                Node {
                    min_width: px(52),
                    max_width: percent(100),
                    ..default()
                },
                TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
                TextContent,
                TextFont::from_font_size(muted_font_size()),
                TextColor(THEME_TEXT),
            )],
        ))
        .id();
    commands.entity(parent).add_child(button);
    button
}

#[derive(Component)]
struct UiIconName(String);

#[derive(Resource, Default)]
struct UiIconAssets(HashMap<String, Handle<Image>>);

fn icon_asset_reference(icon: &str) -> String {
    if icon.starts_with("mod:") || icon.contains('/') {
        icon.to_string()
    } else {
        format!("icons/ui/{icon}.png")
    }
}

/// Resolve semantic icon names into actual PNG sprites from the selected game
/// data folder. Missing images are logged and omitted, never replaced by text.
fn resolve_ui_icons(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiIconAssets>,
    mut logged_missing: Local<HashSet<String>>,
    icons: Query<(Entity, &UiIconName)>,
) {
    for (entity, icon) in &icons {
        let reference = icon_asset_reference(&icon.0);
        if let Some(image) = cache.0.get(&reference) {
            commands
                .entity(entity)
                .insert(ImageNode::new(image.clone()))
                .remove::<UiIconName>();
            continue;
        }

        match crate::asset_loading::load_image_from_data(&mut images, &reference) {
            Ok(image) => {
                cache.0.insert(reference.clone(), image.clone());
                commands
                    .entity(entity)
                    .insert(ImageNode::new(image))
                    .remove::<UiIconName>();
            }
            Err(error) => {
                if logged_missing.insert(reference.clone()) {
                    warn!("Could not load UI icon {reference}: {error}");
                }
                commands.entity(entity).remove::<UiIconName>();
            }
        }
    }
}

/// Spawn a centered image node that resolves on the next presentation update.
pub(crate) fn spawn_icon_image(
    commands: &mut Commands,
    parent: Entity,
    icon: impl Into<String>,
    size: f32,
) -> Entity {
    let entity = commands
        .spawn((
            Node {
                width: px(size),
                height: px(size),
                min_width: px(size),
                min_height: px(size),
                flex_shrink: 0.0,
                ..default()
            },
            UiIconName(icon.into()),
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}

pub fn spawn_action_button(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    icon: impl Into<String>,
    label: impl Into<String>,
) -> Entity {
    let button = commands
        .spawn((
            Button,
            Hovered::default(),
            UiTouchTargetShape::MinimumHeight,
            ChoiceButton { index },
            UiActionButton,
            UiStackedActionButton,
            UiStyledButton,
            action_button_node(),
            BorderColor::all(THEME_BORDER),
            BackgroundColor(THEME_SURFACE),
            children![(
                Node {
                    width: percent(100),
                    min_width: px(0),
                    max_width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: responsive_compact_gap(),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Node {
                            width: px(22),
                            height: px(22),
                            min_width: px(22),
                            min_height: px(22),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        UiIconName(icon.into()),
                    ),
                    (
                        Text::new(label.into()),
                        Node {
                            min_width: px(0),
                            max_width: percent(100),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            ..default()
                        },
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        TextContent,
                        TextFont::from_font_size(label_font_size()),
                        TextColor(THEME_TEXT),
                    ),
                ],
            )],
        ))
        .id();
    commands.entity(parent).add_child(button);
    button
}

pub fn spawn_primary_action_button(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    icon: impl Into<String>,
    label: impl Into<String>,
) -> Entity {
    let button = commands
        .spawn((
            Button,
            Hovered::default(),
            UiTouchTargetShape::MinimumHeight,
            ChoiceButton { index },
            UiActionButton,
            UiStyledButton,
            Node {
                width: Val::Auto,
                min_width: px(110),
                min_height: touch_target_size(),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                padding: UiRect::axes(
                    responsive_vmin(UI_ACTION_PADDING_HORIZONTAL_VMIN),
                    responsive_vmin(UI_ACTION_PADDING_VERTICAL_VMIN),
                ),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(THEME_BORDER),
            BackgroundColor(THEME_SURFACE),
            children![(
                Node {
                    width: percent(100),
                    min_width: px(0),
                    max_width: percent(100),
                    flex_direction: FlexDirection::Row,
                    column_gap: responsive_compact_gap(),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (
                        Node {
                            width: px(22),
                            height: px(22),
                            min_width: px(22),
                            min_height: px(22),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        UiIconName(icon.into()),
                    ),
                    (
                        Text::new(label.into()),
                        Node {
                            min_width: px(0),
                            max_width: percent(100),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            ..default()
                        },
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        TextContent,
                        TextFont::from_font_size(label_font_size()),
                        TextColor(THEME_TEXT),
                    ),
                ],
            )],
        ))
        .id();
    commands.entity(parent).add_child(button);
    button
}

pub fn spawn_menu_button(
    commands: &mut Commands,
    parent: Entity,
    icon: impl Into<String>,
) -> Entity {
    let button = commands
        .spawn((
            Button,
            Hovered::default(),
            UiTouchTargetShape::Square,
            UiMenuButton,
            UiStyledButton,
            Node {
                width: touch_target_size(),
                min_width: touch_target_size(),
                height: touch_target_size(),
                min_height: touch_target_size(),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(THEME_BORDER),
            BackgroundColor(THEME_SURFACE),
            children![(
                Node {
                    width: px(26),
                    height: px(26),
                    ..default()
                },
                UiIconName(icon.into())
            )],
        ))
        .id();
    commands.entity(parent).add_child(button);
    button
}

pub fn spawn_health_gauge(
    commands: &mut Commands,
    parent: Entity,
    current: i32,
    maximum: i32,
) -> Entity {
    let gauge_ratio = gauge_ratio(current, maximum);
    let gauge = commands
        .spawn((
            UiHealthGauge { current, maximum },
            Node {
                width: percent(100),
                min_width: px(0),
                min_height: px(28),
                position_type: PositionType::Relative,
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(THEME_BORDER),
            BackgroundColor(THEME_SURFACE_STRONG),
            children![
                (
                    UiHealthGaugeShadow,
                    Node {
                        width: percent(gauge_ratio * 100.0),
                        height: percent(100),
                        position_type: PositionType::Absolute,
                        left: px(2),
                        top: px(1),
                        ..default()
                    },
                    BackgroundColor(THEME_BLOOD_DARK),
                ),
                (
                    UiHealthGaugeFill,
                    Node {
                        width: percent(gauge_ratio * 100.0),
                        height: percent(100),
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        ..default()
                    },
                    BackgroundColor(THEME_BLOOD),
                ),
                (
                    UiGaugeValueText,
                    Text::new(format!("{current} / {maximum}")),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        min_width: px(0),
                        position_type: PositionType::Absolute,
                        left: px(0),
                        right: px(0),
                        top: px(0),
                        bottom: px(0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    TextContent,
                    TextFont::from_font_size(muted_font_size()),
                    TextColor(THEME_TEXT),
                ),
            ],
        ))
        .id();
    commands.entity(parent).add_child(gauge);
    gauge
}

pub fn spawn_condition_indicator(
    commands: &mut Commands,
    parent: Entity,
    icon: impl Into<String>,
) -> Entity {
    let indicator = commands
        .spawn((
            UiConditionIndicator,
            Node {
                width: touch_target_size(),
                min_width: touch_target_size(),
                height: touch_target_size(),
                min_height: touch_target_size(),
                padding: UiRect::all(responsive_vmin(0.556)),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(THEME_BORDER),
            BackgroundColor(THEME_SURFACE),
            children![(
                Node {
                    width: px(22),
                    height: px(22),
                    ..default()
                },
                UiIconName(icon.into())
            )],
        ))
        .id();
    commands.entity(parent).add_child(indicator);
    indicator
}

pub fn spawn_context_message(
    commands: &mut Commands,
    parent: Entity,
    message: impl Into<String>,
) -> Entity {
    let message = commands
        .spawn((
            UiContextMessage,
            Node {
                width: percent(100),
                min_width: px(0),
                max_width: percent(100),
                padding: UiRect::axes(responsive_surface_padding(), responsive_vmin(1.111)),
                border: UiRect::all(px(1)),
                flex_shrink: 1.0,
                ..default()
            },
            BorderColor::all(THEME_ACCENT),
            BackgroundColor(THEME_SURFACE_STRONG),
            children![(
                Text::new(message.into()),
                Node {
                    width: percent(100),
                    min_width: px(0),
                    max_width: percent(100),
                    ..default()
                },
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                TextContent,
                TextFont::from_font_size(label_font_size()),
                TextColor(THEME_TEXT),
            )],
        ))
        .id();
    commands.entity(parent).add_child(message);
    message
}

fn sync_ui_health_gauges(
    gauges: Query<(&UiHealthGauge, &Children), Changed<UiHealthGauge>>,
    mut fills: Query<&mut Node, (With<UiHealthGaugeFill>, Without<UiHealthGaugeShadow>)>,
    mut shadows: Query<&mut Node, (With<UiHealthGaugeShadow>, Without<UiHealthGaugeFill>)>,
    mut values: Query<&mut Text, With<UiGaugeValueText>>,
) {
    for (gauge, children) in &gauges {
        for child in children.iter() {
            let ratio = gauge_ratio(gauge.current, gauge.maximum);
            if let Ok(mut fill) = fills.get_mut(child) {
                fill.width = percent(ratio * 100.0);
            }
            if let Ok(mut shadow) = shadows.get_mut(child) {
                shadow.width = percent(ratio * 100.0);
            }
            if let Ok(mut value) = values.get_mut(child) {
                *value = Text::new(format!("{} / {}", gauge.current, gauge.maximum));
            }
        }
    }
}

#[allow(clippy::type_complexity)] // ECS query combines widget state and visual components.
fn sync_ui_button_visuals(
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            Option<&UiSelected>,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        With<UiStyledButton>,
    >,
) {
    for (hovered, pressed, selected, mut border, mut background) in &mut buttons {
        let (border_color, background_color) = if pressed {
            (THEME_ACCENT, THEME_PRESSED)
        } else if hovered.get() {
            (THEME_ACCENT, THEME_HOVER)
        } else if selected.is_some() {
            (THEME_ACCENT, THEME_SELECTED)
        } else {
            (THEME_BORDER, THEME_SURFACE)
        };
        border.set_all(border_color);
        *background = BackgroundColor(background_color);
    }
}

pub fn spawn_gauge(commands: &mut Commands, parent: Entity, current: i32, maximum: i32) -> Entity {
    let ratio = gauge_ratio(current, maximum);
    let gauge = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px(0),
                height: px(18),
                ..default()
            },
            BackgroundColor(THEME_PANEL_ALT),
            children![(
                GaugeFill,
                Node {
                    width: percent(ratio * 100.0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(THEME_ACCENT),
            )],
        ))
        .id();
    commands.entity(parent).add_child(gauge);
    gauge
}

pub(crate) fn gauge_ratio(current: i32, maximum: i32) -> f32 {
    if maximum <= 0 {
        0.0
    } else {
        (current.max(0) as f32 / maximum as f32).clamp(0.0, 1.0)
    }
}

fn process_ime_events(
    mut ime: MessageReader<Ime>,
    mut keyboard_input: MessageWriter<KeyboardInput>,
    mut input_focus: ResMut<InputFocus>,
    editable_texts: Query<(), With<EditableText>>,
    mut console_focus: ResMut<ConsoleTextInputFocus>,
) {
    for event in ime.read() {
        match event {
            Ime::Commit { window, value }
                if !cfg!(target_os = "android") && console_focus.active && !value.is_empty() =>
            {
                keyboard_input.write(KeyboardInput {
                    key_code: KeyCode::Unidentified(NativeKeyCode::Unidentified),
                    logical_key: Key::Character(value.clone().into()),
                    state: ButtonState::Pressed,
                    window: *window,
                    repeat: false,
                    text: Some(value.clone().into()),
                });
            }
            Ime::Disabled { .. } if !cfg!(target_os = "android") => {
                if console_focus.active {
                    console_focus.active = false;
                    console_focus.suppress_next_back = true;
                } else if input_focus
                    .get()
                    .is_some_and(|entity| editable_texts.contains(entity))
                {
                    input_focus.clear();
                    console_focus.suppress_next_back = true;
                }
            }
            _ => {}
        }
    }
}

#[cfg(target_os = "android")]
fn android_back_to_semantic_input(
    navigation: Res<NavigationState>,
    input_focus: Res<InputFocus>,
    editable_fields: Query<(), With<LifecycleTextField>>,
    console_focus: Res<ConsoleTextInputFocus>,
    mut lifecycle_queue: ResMut<SemanticInputQueue>,
    mut gameplay_queue: ResMut<GameplayInputQueue>,
) {
    if !ANDROID_BACK_REQUESTED.swap(false, Ordering::AcqRel) {
        return;
    }

    let lifecycle_text_focused = navigation.current_screen == Some(ScreenId::Lifecycle)
        && input_focus
            .get()
            .is_some_and(|entity| editable_fields.contains(entity));

    if lifecycle_text_focused || console_focus.active {
        return;
    }

    if navigation.current_screen == Some(ScreenId::Lifecycle) {
        lifecycle_queue.0.push(InputEvent::Cancel);
    } else {
        gameplay_queue.0.push(InputEvent::Cancel);
    }
}

fn mouse_to_semantic_input(
    mouse: Res<ButtonInput<MouseButton>>,
    navigation: Res<NavigationState>,
    mut lifecycle_queue: ResMut<SemanticInputQueue>,
    mut gameplay_queue: ResMut<GameplayInputQueue>,
) {
    if !mouse.just_pressed(MouseButton::Back) {
        return;
    }

    if navigation.current_screen == Some(ScreenId::Lifecycle) {
        lifecycle_queue.0.push(InputEvent::Cancel);
    } else {
        gameplay_queue.0.push(InputEvent::Cancel);
    }
}

fn keyboard_to_semantic_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    navigation: Res<NavigationState>,
    input_focus: Res<InputFocus>,
    editable_fields: Query<(), With<LifecycleTextField>>,
    mut console_focus: ResMut<ConsoleTextInputFocus>,
    mut lifecycle_queue: ResMut<SemanticInputQueue>,
    mut gameplay_queue: ResMut<GameplayInputQueue>,
) {
    if navigation.current_screen != Some(ScreenId::Console) {
        console_focus.active = false;
    }

    let lifecycle_text_focused = navigation.current_screen == Some(ScreenId::Lifecycle)
        && input_focus
            .get()
            .is_some_and(|entity| editable_fields.contains(entity));

    let mappings = [
        (KeyCode::ArrowUp, InputEvent::Up),
        (KeyCode::ArrowDown, InputEvent::Down),
        (KeyCode::Home, InputEvent::Home),
        (KeyCode::End, InputEvent::End),
        (KeyCode::PageUp, InputEvent::PageUp),
        (KeyCode::PageDown, InputEvent::PageDown),
        (KeyCode::Enter, InputEvent::Confirm),
        (KeyCode::Escape, InputEvent::Cancel),
        (KeyCode::Tab, InputEvent::Tab),
        (KeyCode::Backspace, InputEvent::Backspace),
        (KeyCode::Delete, InputEvent::Delete),
        (KeyCode::BrowserBack, InputEvent::Cancel),
        (KeyCode::KeyJ, InputEvent::Character('j')),
        (KeyCode::KeyK, InputEvent::Character('k')),
        (KeyCode::Digit1, InputEvent::Character('1')),
        (KeyCode::Digit2, InputEvent::Character('2')),
        (KeyCode::Digit3, InputEvent::Character('3')),
    ];
    for (key, event) in mappings {
        if !keyboard.just_pressed(key) {
            continue;
        }
        if lifecycle_text_focused
            && !matches!(key, KeyCode::Enter | KeyCode::Escape | KeyCode::BrowserBack)
        {
            continue;
        }
        if key == KeyCode::BrowserBack {
            if console_focus.suppress_next_back {
                console_focus.suppress_next_back = false;
                continue;
            }
            if console_focus.active || lifecycle_text_focused {
                continue;
            }
        }
        if navigation.current_screen == Some(ScreenId::Lifecycle) {
            lifecycle_queue.0.push(event);
        } else {
            gameplay_queue.0.push(event);
        }
    }
}

// Reproduce keyboard navigation for pointer/touch activation so every choice
// uses the same selection and confirmation path.
pub(crate) fn queue_choice_activation(queue: &mut Vec<InputEvent>, index: usize) {
    queue.push(InputEvent::Home);
    for _ in 0..index {
        queue.push(InputEvent::Down);
    }
    queue.push(InputEvent::Confirm);
}

fn queue_button_activation(
    activation: On<Activate>,
    touches: Res<Touches>,
    scroll_state: Res<TouchScrollState>,
    mut activations: MessageWriter<UiButtonActivation>,
) {
    let button = activation.event_target();
    if scroll_state.should_suppress_activation(button, &touches) {
        return;
    }
    activations.write(UiButtonActivation(button));
}

fn choice_button_input(
    mut activations: MessageReader<UiButtonActivation>,
    choices: Query<(&ChoiceButton, Option<&UiTabButton>)>,
    mut navigation: ResMut<NavigationState>,
    mut lifecycle_queue: ResMut<SemanticInputQueue>,
    mut gameplay_queue: ResMut<GameplayInputQueue>,
) {
    for activation in activations.read() {
        let Ok((choice, tab)) = choices.get(activation.0) else {
            continue;
        };
        if let Some(tab) = tab {
            if navigation.current_screen == Some(ScreenId::Options) {
                gameplay_queue.0.push(InputEvent::SelectTab(tab.index));
            }
            continue;
        }

        navigation.selected = choice.index;
        let queue = if navigation.current_screen == Some(ScreenId::Lifecycle) {
            &mut lifecycle_queue.0
        } else {
            &mut gameplay_queue.0
        };
        queue_choice_activation(queue, choice.index);
    }
}

#[allow(clippy::type_complexity)] // For interaction_query
fn contextual_touch_input(
    mut activations: MessageReader<UiButtonActivation>,
    interaction_query: Query<(
        Entity,
        Option<&LifecycleTextField>,
        Option<&ContextualConsoleInput>,
        Option<&ContextualConsoleTab>,
        Option<&UiMenuButton>,
    )>,
    mut navigation: ResMut<NavigationState>,
    mut input_focus: ResMut<InputFocus>,
    mut console_focus: ResMut<ConsoleTextInputFocus>,
    mut gameplay_queue: ResMut<GameplayInputQueue>,
) {
    for activation in activations.read() {
        let Ok((entity, field, console, tab, menu_button)) = interaction_query.get(activation.0)
        else {
            continue;
        };
        if let Some(field) = field {
            if navigation.current_screen != Some(ScreenId::Lifecycle) {
                continue;
            }
            input_focus.set(entity, FocusCause::Pressed);
            navigation.selected = field.index;
            console_focus.active = false;
            console_focus.suppress_next_back = false;
        } else if console.is_some() {
            if navigation.current_screen == Some(ScreenId::Console) {
                console_focus.active = true;
                console_focus.suppress_next_back = false;
            }
        } else if menu_button.is_some() && navigation.current_screen == Some(ScreenId::Gameplay) {
            gameplay_queue.0.push(InputEvent::OpenSecondaryNavigation);
        } else if tab.is_some() && navigation.current_screen == Some(ScreenId::Console) {
            gameplay_queue.0.push(InputEvent::Tab);
        }
    }
}

fn organize_action_button_groups(
    mut commands: Commands,
    parents: Query<(Entity, &Children, Option<&UiActionButtonGroup>)>,
    action_buttons: Query<(), With<UiStackedActionButton>>,
    action_groups: Query<(), With<UiActionButtonGroup>>,
) {
    for (parent, children, is_action_group) in &parents {
        if is_action_group.is_some() {
            continue;
        }

        let buttons = children
            .iter()
            .filter(|entity| action_buttons.contains(*entity))
            .collect::<Vec<_>>();
        if buttons.is_empty() {
            continue;
        }

        let group = children
            .iter()
            .find(|entity| action_groups.contains(*entity))
            .unwrap_or_else(|| {
                let group = commands
                    .spawn((UiActionButtonGroup, action_button_group_node()))
                    .id();
                commands.entity(parent).add_child(group);
                group
            });

        for button in buttons {
            commands.entity(group).add_child(button);
        }
    }
}

fn touch_scroll(
    touches: Res<Touches>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut state: ResMut<TouchScrollState>,
    panels: Query<(Entity, &ComputedNode, &UiGlobalTransform), With<TouchScrollablePanel>>,
    mut scroll_positions: Query<&mut ScrollPosition, With<TouchScrollablePanel>>,
    buttons: Query<(Entity, &ComputedNode, &UiGlobalTransform), With<Button>>,
) {
    if let Some(cursor_position) = window.cursor_position() {
        let physical_cursor = physical_touch_position(cursor_position, window.scale_factor());
        let wheel_delta: f32 = mouse_wheel.read().map(|event| event.y).sum();
        if wheel_delta != 0.0 {
            for (entity, computed, transform) in &panels {
                if computed.contains_point(*transform, physical_cursor) {
                    if let Ok(mut scroll) = scroll_positions.get_mut(entity) {
                        scroll.0.y = (scroll.0.y - wheel_delta * 48.0).max(0.0);
                    }
                    break;
                }
            }
        }
    } else {
        mouse_wheel.clear();
    }

    for touch in touches.iter_just_pressed() {
        let physical_position = physical_touch_position(touch.position(), window.scale_factor());
        let panel = panels
            .iter()
            .find(|(_, computed, transform)| {
                computed.contains_point(**transform, physical_position)
            })
            .map(|(entity, _, _)| entity);
        let Some(panel) = panel else {
            continue;
        };
        let button = buttons
            .iter()
            .find(|(_, computed, transform)| {
                computed.contains_point(**transform, physical_position)
            })
            .map(|(entity, _, _)| entity);
        state.active = Some(ActiveTouchScroll {
            id: touch.id(),
            panel,
            start: touch.position(),
            last: touch.position(),
            button,
            dragging: false,
        });
    }

    let Some(mut active) = state.active else {
        return;
    };
    if let Some(position) = touches
        .iter()
        .find(|touch| touch.id() == active.id)
        .map(|touch| touch.position())
    {
        let crossed_drag_threshold =
            !active.dragging && touch_scroll_dragged(active.start, position);
        if crossed_drag_threshold {
            active.dragging = true;
        }
        if active.dragging {
            // Apply the full movement once dragging starts so the threshold
            // distance does not feel like lost input.
            let delta = if crossed_drag_threshold {
                position - active.start
            } else {
                position - active.last
            };
            if let Ok(mut scroll) = scroll_positions.get_mut(active.panel) {
                scroll.0.y = (scroll.0.y - delta.y).max(0.0);
            }
        }
        active.last = position;
        state.active = Some(active);
    } else if touches.just_released(active.id) || touches.just_canceled(active.id) {
        state.active = None;
    }
}

#[cfg(not(target_os = "android"))]
fn sync_ime_window(
    input_focus: Res<InputFocus>,
    editable_texts: Query<(), With<EditableText>>,
    console_focus: Res<ConsoleTextInputFocus>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some(mut window) = windows.iter_mut().next() else {
        return;
    };
    let native_editable_focused = input_focus
        .get()
        .is_some_and(|entity| editable_texts.contains(entity));
    window.ime_enabled = console_focus.active || native_editable_focused;
}

#[cfg(target_os = "android")]
fn sync_ime_window() {}

fn sync_lifecycle_field_visuals(
    input_focus: Res<InputFocus>,
    mut fields: Query<(
        Entity,
        &LifecycleTextField,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
) {
    let focused = input_focus
        .get()
        .filter(|entity| fields.get(*entity).is_ok());

    for (entity, _, mut border, mut background) in &mut fields {
        border.set_all(if focused == Some(entity) {
            THEME_ACCENT
        } else {
            THEME_MUTED
        });
        *background = BackgroundColor(THEME_PANEL_ALT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_scroll_requires_a_deliberate_drag() {
        assert!(!touch_scroll_dragged(Vec2::ZERO, Vec2::new(12.0, 0.0)));
        assert!(!touch_scroll_dragged(Vec2::ZERO, Vec2::new(8.0, 8.0)));
        assert!(touch_scroll_dragged(Vec2::ZERO, Vec2::new(12.1, 0.0)));
    }

    #[test]
    fn touch_coordinates_convert_from_logical_to_physical_space() {
        let position = physical_touch_position(Vec2::new(100.0, 50.0), 2.5);
        assert_eq!(position, Vec2::new(250.0, 125.0));
    }

    fn spawn_console_input_regression_fixture(mut commands: Commands) {
        let panel = commands.spawn_empty().id();
        spawn_label(&mut commands, panel, "> help");
        spawn_console_input_row(&mut commands, panel, "> ");
    }

    #[test]
    fn console_history_prefix_does_not_create_an_extra_text_input() {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(spawn_console_input_regression_fixture);
        schedule.run(&mut world);

        let input_count = {
            let mut inputs = world.query_filtered::<Entity, With<ContextualConsoleInput>>();
            inputs.iter(&world).count()
        };
        let tab_count = {
            let mut tabs = world.query_filtered::<Entity, With<ContextualConsoleTab>>();
            tabs.iter(&world).count()
        };
        let command_echo_is_plain_text = {
            let mut labels = world.query::<(&Text, Option<&ContextualConsoleInput>)>();
            labels
                .iter(&world)
                .any(|(text, input)| text.as_str() == "> help" && input.is_none())
        };

        assert_eq!(input_count, 1);
        assert_eq!(tab_count, 1);
        assert!(command_echo_is_plain_text);
    }

    #[test]
    fn navigation_state_starts_without_a_screen_or_selection() {
        let state = NavigationState::default();
        assert_eq!(state.current_screen, None);
        assert_eq!(state.return_screen, None);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn responsive_fonts_use_root_relative_units() {
        assert_eq!(title_font_size(), FontSize::Rem(1.8));
        assert_eq!(label_font_size(), FontSize::Rem(1.0));
        assert_eq!(muted_font_size(), FontSize::Rem(0.9));
    }

    #[test]
    fn ui_theme_exposes_semantic_surface_and_interaction_tokens() {
        assert_eq!(UI_TOUCH_TARGET_PX, 48.0);
        assert_eq!(UI_COMPACT_GAP_VMIN, 1.111);
        assert_eq!(UI_SURFACE_PADDING_VMIN, 1.667);
    }

    #[test]
    fn standalone_labels_retain_intrinsic_height() {
        let node = standalone_label_node();

        assert_eq!(node.min_height, Val::Auto);
        assert_eq!(node.flex_shrink, 0.0);
    }

    #[test]
    fn action_groups_are_bottom_left_and_width_capped() {
        let node = action_button_group_node();

        assert_eq!(node.width, percent(100));
        assert_eq!(node.max_width, px(UI_ACTION_MAX_WIDTH_PX));
        assert_eq!(node.align_self, AlignSelf::FlexStart);
        assert_eq!(node.margin.top, Val::Auto);
    }

    #[test]
    fn stacked_action_buttons_do_not_expand_to_fill_parent() {
        let node = action_button_node();

        assert_eq!(node.flex_grow, 0.0);
        assert_eq!(node.flex_shrink, 0.0);
        assert_eq!(node.min_height, touch_target_size());
    }

    #[test]
    fn option_rows_keep_labels_left_and_controls_right() {
        let label = option_row_label_node();
        let controls = option_row_controls_node();
        let row = option_row_node();
        assert_eq!(label.flex_grow, 1.0);
        assert_eq!(label.align_items, AlignItems::Start);
        assert_eq!(controls.justify_content, JustifyContent::End);
        assert_eq!(row.flex_direction, FlexDirection::Row);
        assert_eq!(row.flex_wrap, FlexWrap::NoWrap);
    }

    #[test]
    fn options_scroll_region_is_separate_from_fixed_panel_and_footer() {
        let panel = fixed_panel_node();
        let list = scrollable_list_node();
        let footer = option_footer_node();
        assert_eq!(panel.flex_grow, 1.0);
        assert_eq!(panel.min_height, px(0));
        assert_eq!(panel.overflow, Overflow::clip());
        assert_eq!(list.flex_grow, 1.0);
        assert_eq!(list.min_height, px(0));
        assert_eq!(list.overflow, Overflow::scroll());
        assert_eq!(footer.flex_shrink, 0.0);
        assert_eq!(footer.flex_direction, FlexDirection::Row);
        assert_eq!(footer.justify_content, JustifyContent::End);
    }

    #[test]
    fn option_rows_create_label_then_right_aligned_controls() {
        #[derive(Resource)]
        struct Fixture(OptionRowSlots);

        fn setup(mut commands: Commands) {
            let parent = commands.spawn_empty().id();
            let row = spawn_option_row(&mut commands, parent);
            commands.insert_resource(Fixture(row));
        }

        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(setup);
        schedule.run(&mut world);

        let slots = world.resource::<Fixture>().0;
        let children = world
            .get::<Children>(slots.row)
            .unwrap()
            .iter()
            .collect::<Vec<_>>();
        assert_eq!(children, vec![slots.label, slots.controls]);
        assert!(world.get::<OptionRowLabelSlot>(slots.label).is_some());
        assert!(world
            .get::<OptionRowControlsSlot>(slots.controls)
            .is_some());
    }

    #[test]
    fn compact_action_buttons_keep_touch_sized_bounds_without_filling_the_row() {
        #[derive(Resource)]
        struct Fixture(Entity);

        fn setup(mut commands: Commands) {
            let parent = commands.spawn_empty().id();
            let button = spawn_compact_action_button(&mut commands, parent, 7, "−5%");
            commands.insert_resource(Fixture(button));
        }

        let node = compact_action_button_node();
        assert_eq!(node.width, Val::Auto);
        assert_eq!(node.min_width, px(76));
        assert_eq!(node.min_height, touch_target_size());
        assert_eq!(node.flex_grow, 0.0);
        assert_eq!(node.flex_shrink, 0.0);

        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(setup);
        schedule.run(&mut world);
        let button = world.resource::<Fixture>().0;
        assert_eq!(
            world.get::<UiTouchTargetShape>(button),
            Some(&UiTouchTargetShape::MinimumBounds {
                width: UI_TOUCH_TARGET_PX as u16,
                height: UI_TOUCH_TARGET_PX as u16,
            })
        );
        assert_eq!(world.get::<ChoiceButton>(button).unwrap().index, 7);
    }

    #[test]
    fn choice_activation_uses_home_down_confirm_sequence() {
        let mut queue = Vec::new();

        queue_choice_activation(&mut queue, 2);

        assert_eq!(
            queue,
            vec![
                InputEvent::Home,
                InputEvent::Down,
                InputEvent::Down,
                InputEvent::Confirm
            ]
        );
    }

    #[test]
    fn gauge_ratio_handles_empty_and_clamps_values() {
        assert_eq!(gauge_ratio(0, 0), 0.0);
        assert_eq!(gauge_ratio(-5, 10), 0.0);
        assert_eq!(gauge_ratio(15, 10), 1.0);
        assert_eq!(gauge_ratio(5, 10), 0.5);
    }
}
