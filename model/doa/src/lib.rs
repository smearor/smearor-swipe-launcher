mod direction;
mod mcp_tools;
mod messages;
mod response;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use direction::DoaDirection;
pub use mcp_tools::DoaMcpResources;
pub use mcp_tools::DoaMcpTools;
pub use messages::command::DoaCommandAction;
pub use messages::command::DoaCommandMessage;
pub use messages::command::TOPIC_COMMAND;
pub use messages::status::DoaStatusMessage;
pub use messages::status::TOPIC_STATUS;
pub use messages::view::DoaView;
pub use response::DoaDirectionResponse;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(DoaStatusMessageConverter, DoaStatusMessage, |json: serde_json::Value| serde_json::from_value(json)
    .unwrap_or_default());

smearor_swipe_launcher_plugin_api::impl_json_convertible!(DoaCommandMessageConverter, DoaCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

/// Register all JSON converter implementations for DoA messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    DoaStatusMessageConverter::register_in_host(context);
    DoaCommandMessageConverter::register_in_host(context);
}
