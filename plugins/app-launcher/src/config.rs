use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct AppLauncherConfig {
    /// The path to the `.desktop` file.
    pub(crate) desktop_file_path: String,
}

impl AppLauncherConfig {
    pub fn parse(config: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config.clone())
    }
}
