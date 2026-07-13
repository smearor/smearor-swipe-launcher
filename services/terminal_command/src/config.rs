use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Signal used for process termination.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum KillSignal {
    Sigterm,
    Sigkill,
}

impl Default for KillSignal {
    fn default() -> Self {
        Self::Sigterm
    }
}

impl KillSignal {
    pub fn to_nix_signal(&self) -> nix::sys::signal::Signal {
        match self {
            Self::Sigterm => nix::sys::signal::Signal::SIGTERM,
            Self::Sigkill => nix::sys::signal::Signal::SIGKILL,
        }
    }
}

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
