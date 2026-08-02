use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

pub const DEFAULT_VOLUME_STEP: f32 = 0.05;

/// Configuration for the audio widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AudioWidgetConfig {
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Volume change step (0.01 to 0.1).
    pub volume_step: f32,
    /// Whether to show the volume bar.
    pub show_volume_bar: bool,
    /// Whether to show the device name label.
    pub show_device_label: bool,
    /// Whether to allow volume over 100%.
    pub allow_overdrive: bool,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Maximum width in characters for the device label.
    pub max_width_chars: i32,
    /// Widget icon configuration (icon_size, icon_only).
    #[serde(flatten)]
    pub icon_config: WidgetIcon,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    /// Widget layout mode (compact or wide).
    pub mode: WidgetMode,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
}

impl AudioWidgetConfig {
    pub fn parse(config: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }

    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

impl Default for AudioWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            volume_step: DEFAULT_VOLUME_STEP,
            show_volume_bar: true,
            show_device_label: true,
            allow_overdrive: false,
            layout: WidgetLayout::default(),
            max_width_chars: 24,
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            actions: ActionBindings::default(),
        }
    }
}
