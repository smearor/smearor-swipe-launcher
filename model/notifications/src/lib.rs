mod mcp;
mod messages;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::requests::NotificationClearArgs;
pub use mcp::requests::NotificationSendArgs;
pub use mcp::requests::NotificationToggleDndArgs;
pub use mcp::resources::NotificationMcpResources;
pub use mcp::responses::NotificationDndResponse;
pub use mcp::responses::NotificationHistoryEntry;
pub use mcp::responses::NotificationHistoryResponse;
pub use mcp::tools::NotificationMcpTools;
pub use messages::command::NotificationCommandAction;
pub use messages::command::NotificationCommandMessage;
pub use messages::command::TOPIC_COMMAND;
pub use messages::status::NotificationAction;
pub use messages::status::NotificationInfo;
pub use messages::status::NotificationStatusMessage;
pub use messages::status::TOPIC_STATUS;
pub use messages::status::UrgencyLevel;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NotificationCommandMessageConverter, NotificationCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(NotificationStatusMessageConverter, NotificationStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register all JSON converter implementations for notifications messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    NotificationCommandMessageConverter::register_in_host(context);
    NotificationStatusMessageConverter::register_in_host(context);
}
