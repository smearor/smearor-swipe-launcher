mod mcp;
mod messages;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::prompts::MprisMcpPrompts;
pub use mcp::requests::MprisSeekArgs;
pub use mcp::requests::MprisSetPositionArgs;
pub use mcp::resources::MprisMcpResources;
pub use mcp::tools::MprisMcpTools;
pub use messages::command::MprisCommandAction;
pub use messages::command::MprisCommandMessage;
pub use messages::command::TOPIC_COMMAND;
pub use messages::status::MprisLoopStatus;
pub use messages::status::MprisPlaybackStatus;
pub use messages::status::MprisPlayerInfo;
pub use messages::status::MprisStatusMessage;
pub use messages::status::MprisTrackMetadata;
pub use messages::status::TOPIC_STATUS;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MprisCommandMessageConverter, MprisCommandMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MprisStatusMessageConverter, MprisStatusMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

/// Register all JSON converter implementations for MPRIS messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    MprisCommandMessageConverter::register_in_host(context);
    MprisStatusMessageConverter::register_in_host(context);
}
