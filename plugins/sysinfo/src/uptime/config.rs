use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

use crate::config::DEFAULT_ICON_SIZE;

/// Display mode for the uptime widget.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub enum UptimeDisplayMode {
    /// Show textual uptime and load average.
    #[default]
    Info,
    /// Show a circular gauge with the uptime in the center.
    Gauge,
}

/// Configuration for the uptime widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct UptimeWidgetConfig {
    /// Visual display mode.
    pub display_mode: UptimeDisplayMode,
    /// Format string for the uptime label.
    pub value_format: String,
    /// Whether to show the uptime as a human-readable duration.
    pub show_uptime: bool,
    /// Whether to show the 1-minute load average.
    pub show_load_average_1_minute: bool,
    /// Whether to show the 5-minute load average.
    pub show_load_average_5_minute: bool,
    /// Whether to show the 15-minute load average.
    pub show_load_average_15_minute: bool,
    /// Whether to show an icon.
    pub show_icon: bool,
    /// Optional icon name.
    pub icon: Option<String>,
    /// Size of the icon in pixels.
    pub icon_size: i32,
    /// Widget dimensions (width, height, scale) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
}

impl Default for UptimeWidgetConfig {
    fn default() -> Self {
        Self {
            display_mode: UptimeDisplayMode::Info,
            value_format: String::from("{value}"),
            show_uptime: true,
            show_load_average_1_minute: true,
            show_load_average_5_minute: true,
            show_load_average_15_minute: true,
            show_icon: true,
            icon: Some(String::from("nf-md-clock_start")),
            icon_size: DEFAULT_ICON_SIZE,
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            text_colors: WidgetTextColors::default(),
            actions: ActionBindings::default(),
        }
    }
}

impl UptimeWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
