use serde::Deserialize;
use serde::Serialize;

/// Configuration for an application-based wallpaper theme.
/// The service spawns the configured command with its arguments on the Layer Shell background layer.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AppConfig {
    /// Base executable command (e.g., "smearor-wrot").
    pub command: String,
    /// Target display outputs. `["ALL"]` targets all monitors; otherwise specific names like `["DP-1", "HDMI-A-1"]`.
    pub outputs: Vec<String>,
    /// Array of arguments passed to the command.
    /// The placeholder `{monitor}` is replaced at runtime with each target monitor name,
    /// spawning one process per output.
    pub arguments: Vec<String>,
}
