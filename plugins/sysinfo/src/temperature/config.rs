use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

use crate::config::DEFAULT_ICON_SIZE;

/// Configuration for the temperature widget.
///
/// Displays one or more temperature components as circular gauges.
/// Each gauge shows current temperature, max marker, and critical threshold.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct TemperatureWidgetConfig {
    /// Filter list for which temperature components to display.
    ///
    /// Each entry is matched against component label and id (case-insensitive substring).
    /// If empty, all available components are shown.
    pub components: Vec<String>,
    /// Format string for the temperature label.
    pub format: String,
    /// Whether to show the component label text.
    pub show_label: bool,
    /// Gauge diameter in pixels.
    pub gauge_size: i32,
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

impl Default for TemperatureWidgetConfig {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            format: String::from("{temperature:.0}°C"),
            show_label: true,
            gauge_size: 120,
            show_icon: true,
            icon: Some(String::from("nf-md-thermometer")),
            icon_size: DEFAULT_ICON_SIZE,
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            text_colors: WidgetTextColors::default(),
            actions: ActionBindings::default(),
        }
    }
}

impl TemperatureWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
