use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `app_launcher_exec` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct AppLauncherExecArgs {
    /// Canonical path to the .desktop file
    pub desktop_file: String,
    /// Whether the process should be detached from the launcher (default: false)
    pub forked: Option<bool>,
    /// Whether to terminate the process when the launcher exits (default: true)
    pub terminate_on_exit: Option<bool>,
}

/// Arguments for the `app_launcher_terminate` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct AppLauncherTerminateArgs {
    /// Canonical path to the .desktop file
    pub desktop_file: String,
}

/// Arguments for the `app_launcher_search_apps` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct AppLauncherSearchAppsArgs {
    /// Search query (e.g., 'calculator', 'browser', 'game', 'audio'). Matches against app name, generic name, comment, keywords, and categories case-insensitively.
    pub query: String,
}
