use serde::Deserialize;
use smearor_wrot_process::KillSignal;
use std::collections::HashMap;
use std::path::PathBuf;

/// A configured command definition.
#[derive(Debug, Clone, Deserialize)]
pub struct CommandDefinition {
    /// Absolute path to a binary or a name resolvable via `$PATH`.
    pub command: String,
    /// Ordered list of arguments passed to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Additional environment variables merged into the spawned process environment.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Optional working directory for the spawned process.
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    /// If `true`, the service restarts the command if it exits unexpectedly.
    #[serde(default)]
    pub restart_on_exit: bool,
    /// Signal used for termination.
    #[serde(default)]
    pub kill_signal: KillSignal,
    /// Grace period in milliseconds before escalating to `SIGKILL`.
    #[serde(default = "default_terminate_timeout_ms")]
    pub terminate_timeout_ms: u64,
}

fn default_terminate_timeout_ms() -> u64 {
    2000
}

/// Configuration for the terminal command service.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TerminalCommandServiceConfig {
    /// Configured commands, keyed by command_id.
    #[serde(default)]
    pub commands: HashMap<String, CommandDefinition>,
}
