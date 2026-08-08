use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `terminal_command_launch` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct TerminalCommandLaunchArgs {
    /// The configured command identifier
    pub command_id: String,
    /// Whether the process should be detached from the launcher (default: false)
    pub forked: Option<bool>,
    /// Whether to terminate the process when the launcher exits (default: true)
    pub terminate_on_exit: Option<bool>,
}

/// Arguments for the `terminal_command_terminate` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct TerminalCommandTerminateArgs {
    /// The configured command identifier
    pub command_id: String,
}

/// Arguments for the `terminal_command_restart` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct TerminalCommandRestartArgs {
    /// The configured command identifier
    pub command_id: String,
    /// Whether the process should be detached from the launcher (default: false)
    pub forked: Option<bool>,
    /// Whether to terminate the process when the launcher exits (default: true)
    pub terminate_on_exit: Option<bool>,
}
