use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for status broadcast messages.
pub const TOPIC_STATUS: &str = "service.terminal_command.status";

/// Status of a tracked terminal command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Debug, Clone, Default)]
pub enum TerminalCommandStatus {
    /// The command has been spawned and is still active.
    #[default]
    Running,
    /// The command is no longer running (terminated, exited, or crashed).
    Stopped,
    /// The command could not be started (e.g. binary not found, permission denied).
    Failed,
}

/// Status broadcast message published by the terminal command service.
#[derive(Debug, Clone, Default)]
pub struct TerminalCommandStatusMessage {
    /// The configured command identifier.
    pub command_id: String,
    /// The new status of the command.
    pub status: TerminalCommandStatus,
    /// The PID of the running process, if available.
    pub pid: Option<u32>,
}

/// ABI-stable version of `TerminalCommandStatusMessage` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default)]
pub struct TerminalCommandStatusMessageStabby {
    pub command_id: stabby::string::String,
    pub status: TerminalCommandStatus,
    pub pid: stabby::option::Option<u32>,
}

impl From<TerminalCommandStatusMessage> for TerminalCommandStatusMessageStabby {
    fn from(value: TerminalCommandStatusMessage) -> Self {
        Self {
            command_id: value.command_id.into(),
            status: value.status,
            pid: value.pid.into(),
        }
    }
}

impl From<TerminalCommandStatusMessageStabby> for TerminalCommandStatusMessage {
    fn from(value: TerminalCommandStatusMessageStabby) -> Self {
        Self {
            command_id: value.command_id.to_string(),
            status: value.status,
            pid: value.pid.into(),
        }
    }
}

impl TerminalCommandStatusMessage {
    pub fn new(command_id: &str, status: TerminalCommandStatus, pid: Option<u32>) -> Self {
        Self {
            command_id: command_id.to_string(),
            status,
            pid,
        }
    }

    pub fn running(command_id: &str, pid: u32) -> Self {
        Self::new(command_id, TerminalCommandStatus::Running, Some(pid))
    }

    pub fn stopped(command_id: &str) -> Self {
        Self::new(command_id, TerminalCommandStatus::Stopped, None)
    }

    pub fn failed(command_id: &str) -> Self {
        Self::new(command_id, TerminalCommandStatus::Failed, None)
    }
}

impl TypedMessage for TerminalCommandStatusMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_terminal_command_model::TerminalCommandStatusMessageStabby");
}

impl TypedMessage for TerminalCommandStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_terminal_command_model::TerminalCommandStatusMessage");
}

impl MessageTopic for TerminalCommandStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl MessageTopic for TerminalCommandStatusMessageStabby {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for TerminalCommandStatusMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
