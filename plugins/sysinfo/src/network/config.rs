use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

use crate::config::DEFAULT_ICON_SIZE;

/// Display mode for the network widget.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub enum NetworkDisplayMode {
    /// Show textual up/down values.
    #[default]
    Info,
    /// Show a circular gauge with two semicircles for up/down.
    Gauge,
}

/// Configuration for the network widget.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct NetworkWidgetConfig {
    /// Visual display mode.
    pub display_mode: NetworkDisplayMode,
    /// Whether to show received bytes per second.
    pub show_received: bool,
    /// Whether to show transmitted bytes per second.
    pub show_transmitted: bool,
    /// Maximum expected received bytes per second for the gauge (0 means auto).
    pub max_download: f64,
    /// Maximum expected transmitted bytes per second for the gauge (0 means auto).
    pub max_upload: f64,
    /// Whether to show a small sparkline history.
    pub show_history: bool,
    /// Number of history samples to keep.
    pub history_length: usize,
    /// Whether to show an icon.
    pub show_icon: bool,
    /// Optional icon name.
    pub icon: Option<String>,
    /// Size of the icon in pixels.
    pub icon_size: i32,
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

impl Default for NetworkWidgetConfig {
    fn default() -> Self {
        Self {
            display_mode: NetworkDisplayMode::Info,
            show_received: true,
            show_transmitted: true,
            max_download: 0.0,
            max_upload: 0.0,
            show_history: false,
            history_length: 30,
            show_icon: true,
            icon: Some(String::from("nf-md-network")),
            icon_size: DEFAULT_ICON_SIZE,
            layout: WidgetLayout::default(),
            text_colors: WidgetTextColors::default(),
            actions: ActionBindings::default(),
        }
    }
}

impl NetworkWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
