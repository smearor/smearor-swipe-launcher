use serde::Deserialize;
use serde_json::Value;

pub const DEFAULT_WIDTH: i32 = 100;

pub const DEFAULT_ICON_SIZE: i32 = 48;

#[derive(Debug, Deserialize)]
pub struct AppLauncherConfig {
    /// The path to the `.desktop` file.
    pub(crate) desktop_file_path: String,
    /// Button width
    #[serde(default = "default_width")]
    pub width: i32,
    /// Icon size
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
    /// Show only icon without text
    #[serde(default)]
    pub icon_only: bool,
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
