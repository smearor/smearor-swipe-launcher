mod device;
mod mcp;
mod messages;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use device::AudioDevice;
pub use mcp::resources::AudioMcpResources;
pub use mcp::tools::AudioMcpTools;
pub use mcp::widget_action::AudioWidgetAction;
pub use messages::command::AudioCommandAction;
pub use messages::command::AudioCommandMessage;
pub use messages::command::TOPIC_COMMAND;
pub use messages::status::AudioStatusMessage;
pub use messages::status::TOPIC_STATUS;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AudioCommandMessageConverter, AudioCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

smearor_swipe_launcher_plugin_api::impl_json_convertible!(AudioStatusMessageConverter, AudioStatusMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

/// Register all JSON converter implementations for audio messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    AudioCommandMessageConverter::register_in_host(context);
    AudioStatusMessageConverter::register_in_host(context);
}
