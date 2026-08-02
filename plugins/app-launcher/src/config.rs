use serde::Deserialize;
use serde::Serialize;
use smearor_app_launcher_model::SmearorWindowRotationWrapper;
use smearor_swipe_launcher_plugin_api::ActionBindings;
use smearor_swipe_launcher_plugin_api::ActionKind;
use smearor_swipe_launcher_plugin_api::DispatchableBinding;
use smearor_swipe_launcher_plugin_api::WidgetDimensions;
use smearor_swipe_launcher_plugin_api::WidgetIcon;
use smearor_swipe_launcher_plugin_api::WidgetLayout;
use smearor_swipe_launcher_plugin_api::WidgetMode;
use smearor_swipe_launcher_plugin_api::WidgetTextColors;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppLauncherConfig {
    /// The path to the `.desktop` file.
    pub desktop_file_path: String,
    /// Optional override for the icon name.
    #[serde(default)]
    pub icon: Option<String>,
    /// The smearor window rotation wrapper configuration
    pub wrapper: Option<SmearorWindowRotationWrapper>,
    /// Widget dimensions (width, height) for GTK layout.
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    /// Widget icon configuration (icon_size, icon_only).
    #[serde(flatten)]
    pub icon_config: WidgetIcon,
    /// Human-readable description of what the app launcher does.
    #[serde(default)]
    pub description: Option<String>,
    /// Action bindings for all input triggers.
    #[serde(flatten)]
    pub actions: ActionBindings,
    /// Widget layout (spacing) for GTK container.
    #[serde(flatten)]
    pub layout: WidgetLayout,
    /// Text color configuration (main_text_color, info_text_color).
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,
    /// Widget layout mode (compact or wide).
    /// Compact: vertical layout (icon on top, app name below).
    /// Wide: horizontal layout (icon on left, app name on right).
    #[serde(default)]
    pub mode: WidgetMode,
    /// Whether the process should be detached (forked) from the launcher.
    /// Forked processes survive launcher exit and cannot be terminated via long-press.
    #[serde(default)]
    pub forked: bool,
    /// Whether to terminate the tracked process when the launcher exits.
    /// Only applies to non-forked processes. Defaults to true.
    #[serde(default = "default_terminate_on_exit")]
    pub terminate_on_exit: bool,
}

impl AppLauncherConfig {
    pub fn parse(config: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }

    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        self.actions.binding_for_kind(kind)
    }
}

fn default_terminate_on_exit() -> bool {
    true
}
