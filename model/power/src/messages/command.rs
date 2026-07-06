use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::PowerAction;

pub const TOPIC_COMMAND: &str = "service.power.command";

/// Actions the power service can perform on request.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PowerCommandAction {
    /// Execute a power action immediately (with countdown if configured).
    #[default]
    Execute,
    /// Schedule a power action for the future.
    Schedule,
    /// Cancel a running countdown or scheduled action.
    Cancel,
    /// Refresh capabilities and inhibitors from the system.
    Refresh,
}

/// Command message sent by widgets or MCP clients to the power service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct PowerCommandMessage {
    /// The action to execute.
    pub action: PowerCommandAction,
    /// The power action to perform (ignored for `Cancel` and `Refresh`).
    pub power_action: PowerAction,
    /// Delay in minutes for scheduled actions (only used with `Schedule`).
    pub delay_minutes: u32,
}

impl PowerCommandMessage {
    /// Creates a new power command message.
    pub fn new(action: PowerCommandAction, power_action: PowerAction, delay_minutes: u32) -> Self {
        Self {
            action,
            power_action,
            delay_minutes,
        }
    }

    /// Creates an execute command for the given power action.
    pub fn execute(power_action: PowerAction) -> Self {
        Self::new(PowerCommandAction::Execute, power_action, 0)
    }

    /// Creates a schedule command for the given power action with a delay in minutes.
    pub fn schedule(power_action: PowerAction, delay_minutes: u32) -> Self {
        Self::new(PowerCommandAction::Schedule, power_action, delay_minutes)
    }

    /// Creates a cancel command.
    pub fn cancel() -> Self {
        Self::new(PowerCommandAction::Cancel, PowerAction::Cancel, 0)
    }

    /// Creates a refresh command.
    pub fn refresh() -> Self {
        Self::new(PowerCommandAction::Refresh, PowerAction::Cancel, 0)
    }
}

impl TypedMessage for PowerCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_power_model::PowerCommandMessage");
}

impl MessageTopic for PowerCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for PowerCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
