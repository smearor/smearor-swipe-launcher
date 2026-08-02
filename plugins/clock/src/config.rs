use serde::Deserialize;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;
use typed_builder::TypedBuilder;

/// Configuration for the clock widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct ClockConfig {
    /// The timezone (e.g. "local", "utc"). Defaults to local time.
    #[builder(default, setter(into))]
    pub(crate) timezone: Option<String>,
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) dimensions: WidgetDimensions,
    /// Widget icon configuration (icon_size, icon_only).
    /// For the clock widget, `icon_size` controls the font size of the time display.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) icon_config: WidgetIcon,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) text_colors: WidgetTextColors,
    /// Widget layout mode (compact or wide).
    /// Compact: shows HH:MM. Wide: shows HH:MM:SS.
    #[serde(default)]
    pub(crate) mode: WidgetMode,
    /// The background color of the widget.
    #[builder(default, setter(into))]
    pub(crate) background_color: Option<String>,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    #[builder(default)]
    pub actions: ActionBindings,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    #[builder(default)]
    pub(crate) layout: WidgetLayout,
    /// Human-readable description of what the clock widget does.
    #[serde(default)]
    pub description: Option<String>,
}

impl ClockConfig {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            timezone: None,
            dimensions: WidgetDimensions::default(),
            icon_config: WidgetIcon::default(),
            text_colors: WidgetTextColors::default(),
            mode: WidgetMode::default(),
            background_color: None,
            actions: ActionBindings::default(),
            layout: WidgetLayout::default(),
            description: None,
        }
    }
}
