mod mcp;
mod messages;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::prompts::PowerMcpPrompts;
pub use mcp::resources::PowerMcpResources;
pub use mcp::tools::PowerMcpTools;
pub use messages::capabilities::PowerCapabilities;
pub use messages::command::PowerCommandAction;
pub use messages::command::PowerCommandMessage;
pub use messages::command::TOPIC_COMMAND;
pub use messages::icon::power_action_icon;
pub use messages::icon::power_action_icon_unicode;
pub use messages::inhibitor::InhibitorInfo;
pub use messages::power_action::PowerAction;
pub use messages::scheduled::ScheduledActionInfo;
pub use messages::status::PowerStatusMessage;
pub use messages::status::TOPIC_STATUS;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PowerCommandMessageConverter, PowerCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

smearor_swipe_launcher_plugin_api::impl_json_convertible!(PowerStatusMessageConverter, PowerStatusMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

/// Register all JSON converter implementations for power messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    PowerCommandMessageConverter::register_in_host(context);
    PowerStatusMessageConverter::register_in_host(context);
}
