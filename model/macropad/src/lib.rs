//! Shared message types for MacroPad device integration.
//!
//! This crate defines the message types and topics used by MacroPad services
//! (Stream Deck, Loupedeck) to communicate with the host and other instances
//! via the message broker.
//!
//! # Topics
//!
//! - `service.macropad.input` — Input events from MacroPad devices
//! - `service.macropad.connection` — Connection status updates
//! - `service.macropad.command` — Commands sent to MacroPad devices

mod command_message;
mod connection_status;
mod device_command;
mod dimming_config;
mod dimming_phase;
mod dimming_state;
mod input_message;
mod mcp;
mod topics;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

pub use command_message::MacroPadCommand;
pub use command_message::MacroPadCommandMessage;
pub use command_message::MacroPadCommandType;
pub use connection_status::MacroPadConnectionStatus;
pub use device_command::DeviceCommand;
pub use dimming_config::DimmingConfig;
pub use dimming_config::DimmingConfigOverride;
pub use dimming_phase::DimmingPhase;
pub use dimming_state::DimmingState;
pub use input_message::MacroPadInputMessage;
pub use mcp::tools::MacroPadMcpTools;
pub use topics::TOPIC_MACROPAD_COMMAND;
pub use topics::TOPIC_MACROPAD_CONNECTION;
pub use topics::TOPIC_MACROPAD_INPUT;

impl_json_convertible!(MacroPadInputMessageConverter, MacroPadInputMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());
impl_json_convertible!(MacroPadConnectionStatusConverter, MacroPadConnectionStatus, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());
impl_json_convertible!(MacroPadCommandMessageConverter, MacroPadCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

/// Register all JSON converter implementations for MacroPad messages.
///
/// Call this once during startup.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    MacroPadInputMessageConverter::register_in_host(context);
    MacroPadConnectionStatusConverter::register_in_host(context);
    MacroPadCommandMessageConverter::register_in_host(context);
}
