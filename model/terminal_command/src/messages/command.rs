use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for command control messages (launch, terminate, restart).
pub const TOPIC_COMMAND: &str = "service.terminal_command.command";

/// Action to perform on a configured terminal command.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum TerminalCommandAction {
    /// Launch the configured command.
    #[default]
    Launch,
    /// Terminate the running command.
    Terminate,
    /// Restart the command (terminate then launch).
    Restart,
}

/// Command control message sent by widgets to the terminal command service.
#[derive(Clone, Debug, Default)]
pub struct TerminalCommandMessage {
    /// The configured command identifier to act upon.
    pub command_id: String,
    /// The action to perform.
    pub action: TerminalCommandAction,
    /// Whether the process should be detached (forked) from the launcher.
    /// Forked processes survive launcher exit and cannot be terminated via long-press.
    pub forked: bool,
    /// Whether to terminate the tracked process when the launcher exits.
    /// Only applies to non-forked processes. Defaults to true.
    pub terminate_on_exit: bool,
}

/// ABI-stable version of `TerminalCommandMessage` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct TerminalCommandMessageStabby {
    pub command_id: stabby::string::String,
    pub action: TerminalCommandAction,
    pub forked: bool,
    pub terminate_on_exit: bool,
}

impl From<TerminalCommandMessage> for TerminalCommandMessageStabby {
    fn from(value: TerminalCommandMessage) -> Self {
        Self {
            command_id: value.command_id.into(),
            action: value.action,
            forked: value.forked,
            terminate_on_exit: value.terminate_on_exit,
        }
    }
}

impl From<TerminalCommandMessageStabby> for TerminalCommandMessage {
    fn from(value: TerminalCommandMessageStabby) -> Self {
        Self {
            command_id: value.command_id.to_string(),
            action: value.action,
            forked: value.forked,
            terminate_on_exit: value.terminate_on_exit,
        }
    }
}

impl TerminalCommandMessage {
    pub fn new(command_id: &str, action: TerminalCommandAction, forked: bool, terminate_on_exit: bool) -> Self {
        Self {
            command_id: command_id.to_string(),
            action,
            forked,
            terminate_on_exit,
        }
    }

    pub fn launch(command_id: &str, forked: bool, terminate_on_exit: bool) -> Self {
        Self::new(command_id, TerminalCommandAction::Launch, forked, terminate_on_exit)
    }

    pub fn terminate(command_id: &str) -> Self {
        Self::new(command_id, TerminalCommandAction::Terminate, false, false)
    }

    pub fn restart(command_id: &str, forked: bool, terminate_on_exit: bool) -> Self {
        Self::new(command_id, TerminalCommandAction::Restart, forked, terminate_on_exit)
    }
}

impl TypedMessage for TerminalCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_terminal_command_model::TerminalCommandMessageStabby");
}

impl TypedMessage for TerminalCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_terminal_command_model::TerminalCommandMessage");
}

impl MessageTopic for TerminalCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl MessageTopic for TerminalCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for TerminalCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
