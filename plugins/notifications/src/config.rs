use serde::Deserialize;
use serde_json::Value;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DEFAULT_ICON_SIZE;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMetadata;

/// Configuration for the notification widget.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NotificationWidgetConfig {
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Maximum number of notifications to display.
    pub max_visible: usize,
    /// Whether to show the Do Not Disturb toggle.
    pub show_dnd_toggle: bool,
    /// Whether to show notification icons.
    pub show_icons: bool,
    /// Size of notification icons in pixels.
    pub icon_size: i32,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Widget metadata (description for MCP tool registration).
    #[serde(flatten)]
    pub metadata: WidgetMetadata,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
}

impl NotificationWidgetConfig {
    pub fn parse(config: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

impl Default for NotificationWidgetConfig {
    fn default() -> Self {
        Self {
            dimensions: WidgetDimensions::default(),
            max_visible: 3,
            show_dnd_toggle: true,
            show_icons: true,
            icon_size: DEFAULT_ICON_SIZE,
            layout: WidgetLayout::default(),
            metadata: WidgetMetadata::default(),
            actions: ActionBindings::default(),
        }
    }
}

impl NotificationWidgetConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}
