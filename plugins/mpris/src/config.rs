use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMetadata;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

/// Configuration for the MPRIS widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MprisWidgetConfig {
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Whether to show the album art.
    pub show_album_art: bool,
    /// Whether to show the progress bar.
    pub show_progress_bar: bool,
    /// Whether to show the player name label.
    pub show_player_label: bool,
    /// List of allowed player bus names (empty = all players).
    pub player_filter: Vec<String>,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Maximum width in characters for title and artist labels.
    pub max_width_chars: i32,
    /// Widget icon configuration (icon_size, icon_only).
    #[serde(flatten)]
    pub icon_config: WidgetIcon,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    /// Widget layout mode (compact or wide).
    pub mode: WidgetMode,
    /// Widget metadata (description for MCP tool registration).
    #[serde(flatten)]
    pub metadata: WidgetMetadata,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
}

impl MprisWidgetConfig {
    pub fn parse(config: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }

    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

impl Default for MprisWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            show_album_art: true,
            show_progress_bar: true,
            show_player_label: true,
            player_filter: Vec::new(),
            layout: WidgetLayout::default(),
            max_width_chars: 24,
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            metadata: WidgetMetadata::default(),
            actions: ActionBindings::default(),
        }
    }
}
