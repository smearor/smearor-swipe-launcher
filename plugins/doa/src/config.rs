use serde::Deserialize;
use smearor_doa_model::DoaView;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

pub const DEFAULT_ICON_COMPASS: &str = "nf-md-compass";

pub const DEFAULT_ICON_DIRECTION_NORTH: &str = "nf-md-arrow_up";

pub const DEFAULT_ICON_DIRECTION_EAST: &str = "nf-md-arrow_right";

pub const DEFAULT_ICON_DIRECTION_SOUTH: &str = "nf-md-arrow_down";

pub const DEFAULT_ICON_DIRECTION_WEST: &str = "nf-md-arrow_left";

pub const DEFAULT_ICON_DISCONNECTED: &str = "nf-md-compass_off";

pub const DEFAULT_ICON_DEVICE: &str = "nf-md-microphone_variant";

pub const DEFAULT_ICON_SPEECH: &str = "nf-md-account_voice";

/// Configuration for the DoA widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DoaWidgetConfig {
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Widget icon configuration (icon_size, icon_only).
    #[serde(flatten)]
    pub icon_config: WidgetIcon,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    /// Widget layout mode (compact or wide).
    pub mode: WidgetMode,
    /// Compass view icon.
    pub icon_compass: String,
    /// Direction view icon for North.
    pub icon_direction_north: String,
    /// Direction view icon for East.
    pub icon_direction_east: String,
    /// Direction view icon for South.
    pub icon_direction_south: String,
    /// Direction view icon for West.
    pub icon_direction_west: String,
    /// Disconnected state icon.
    pub icon_disconnected: String,
    /// Device info view icon.
    pub icon_device: String,
    /// Speech activity icon.
    pub icon_speech: String,
    /// Views to cycle through on swipe up/down.
    pub views: Vec<DoaView>,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
}

impl DoaWidgetConfig {
    pub fn parse(config: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }

    /// Returns the icon name for the given DoA direction.
    pub fn direction_icon(&self, direction: &smearor_doa_model::DoaDirection) -> &str {
        match direction {
            smearor_doa_model::DoaDirection::North => &self.icon_direction_north,
            smearor_doa_model::DoaDirection::East => &self.icon_direction_east,
            smearor_doa_model::DoaDirection::South => &self.icon_direction_south,
            smearor_doa_model::DoaDirection::West => &self.icon_direction_west,
        }
    }
}

impl Default for DoaWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            layout: WidgetLayout::default(),
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            icon_compass: DEFAULT_ICON_COMPASS.to_string(),
            icon_direction_north: DEFAULT_ICON_DIRECTION_NORTH.to_string(),
            icon_direction_east: DEFAULT_ICON_DIRECTION_EAST.to_string(),
            icon_direction_south: DEFAULT_ICON_DIRECTION_SOUTH.to_string(),
            icon_direction_west: DEFAULT_ICON_DIRECTION_WEST.to_string(),
            icon_disconnected: DEFAULT_ICON_DISCONNECTED.to_string(),
            icon_device: DEFAULT_ICON_DEVICE.to_string(),
            icon_speech: DEFAULT_ICON_SPEECH.to_string(),
            views: vec![DoaView::Compass, DoaView::Direction, DoaView::DeviceInfo],
            actions: ActionBindings::default(),
        }
    }
}
