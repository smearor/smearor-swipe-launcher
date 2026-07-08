use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use smearor_app_launcher_model::SmearorWindowRotationWrapper;

pub const DEFAULT_WIDTH: i32 = 100;

pub const DEFAULT_ICON_SIZE: i32 = 36;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppLauncherConfig {
    /// The path to the `.desktop` file.
    pub desktop_file_path: String,
    /// Optional override for the icon name.
    #[serde(default)]
    pub icon: Option<String>,
    /// The smearor window rotation wrapper configuration
    pub wrapper: Option<SmearorWindowRotationWrapper>,
    /// Button width
    #[serde(default = "default_width")]
    pub width: i32,
    /// Icon size
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
    /// Show only icon without text
    #[serde(default)]
    pub icon_only: bool,
    /// Message topic for single-click
    #[serde(default)]
    pub click_topic: Option<String>,
    /// Message payload for single-click (JSON/TOML)
    #[serde(default)]
    pub click_payload: Option<Value>,
    /// Target instance for single-click message
    #[serde(default)]
    pub click_instance: Option<String>,
    /// Message topic for long-press
    #[serde(default)]
    pub longpress_topic: Option<String>,
    /// Message payload for long-press (JSON/TOML)
    #[serde(default)]
    pub longpress_payload: Option<Value>,
    /// Target instance for long-press message
    #[serde(default)]
    pub longpress_instance: Option<String>,
    /// Spacing between child widgets inside the app launcher button.
    #[serde(default)]
    pub spacing: i32,
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
    pub fn parse(config: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}

fn default_width() -> i32 {
    DEFAULT_WIDTH
}

fn default_icon_size() -> i32 {
    DEFAULT_ICON_SIZE
}

fn default_terminate_on_exit() -> bool {
    true
}
