use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

/// Configuration for a button widget
#[derive(Debug, Clone, Deserialize)]
pub struct ButtonConfig {
    /// Primary label text (hidden if icon_only is true)
    #[serde(default)]
    pub main_text: String,
    /// Secondary info text displayed below the main label.
    /// Shown in a smaller font. Empty string hides it.
    #[serde(default)]
    pub info_text: String,
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Icon name from icon theme
    #[serde(default)]
    pub icon: Option<String>,
    /// Widget icon configuration (icon_size, icon_only).
    #[serde(flatten)]
    pub icon_config: WidgetIcon,
    /// Tooltip text on hover
    #[serde(default)]
    pub tooltip: Option<String>,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
    /// Keyboard shortcut (e.g., "Ctrl+G", "Alt+F1")
    #[serde(default)]
    pub shortcut: Option<String>,
    /// Whether the button is interactive
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Whether the button is in active state
    #[serde(default)]
    pub active: bool,
    /// Animation type on button press (scale, fade, ripple)
    #[serde(default)]
    pub press_animation: Option<String>,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    /// Topic whose messages control the label text.
    #[serde(default)]
    pub label_topic: Option<String>,
    /// Format string for the label display (JSON values via serde_json).
    #[serde(default)]
    pub label_format: Option<String>,
    /// Fallback text when the topic has not yet delivered a message.
    #[serde(default)]
    pub label_fallback: Option<String>,
    /// Topic whose messages update the internal state (JSON)
    #[serde(default)]
    pub state_topic: Option<String>,
    /// Icon expression evaluated against the internal state.
    /// Supports static icon names and conditional expressions like "{ison?nf-md-fan:nf-md-fan-off}".
    #[serde(default)]
    pub state_icon: Option<String>,
    /// CSS class added when the internal state is truthy, removed when falsy.
    #[serde(default)]
    pub state_css_class: Option<String>,
    /// Label format string evaluated against the internal state.
    #[serde(default)]
    pub state_label: Option<String>,
    /// Optional description for MCP tool registration. When set, the button
    /// registers an MCP tool that allows the voice assistant to trigger actions.
    /// The tool supports an "action" parameter: "click", "longpress", "hold_start", "hold_stop", "double_press", "swipe_up", "swipe_down", "right_click", "middle_click", "scroll_up", "scroll_down", "compound_longpress".
    #[serde(default)]
    pub description: Option<String>,
}

impl ButtonConfig {
    pub fn parse(config: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }

    /// Dispatches an action kind via the broadcaster, respecting `instance`.
    ///
    /// Returns `true` if the action was configured and dispatched, `false` otherwise.
    pub fn dispatch_by_kind(&self, kind: ActionKind, broadcaster: &smearor_swipe_launcher_plugin_api::MessageBroadcasterInner) -> bool {
        self.actions.dispatch_by_kind(kind, broadcaster)
    }
}

fn default_enabled() -> bool {
    true
}
