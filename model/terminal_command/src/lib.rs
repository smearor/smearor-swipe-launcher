mod mcp;
mod messages;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;

pub use mcp::prompts::TerminalCommandMcpPrompts;
pub use mcp::resources::TerminalCommandMcpResources;
pub use mcp::tools::TerminalCommandMcpTools;
pub use messages::command::TerminalCommandAction;
pub use messages::command::TerminalCommandMessage;
pub use messages::command::TerminalCommandMessageStabby;
pub use messages::status::TerminalCommandStatus;
pub use messages::status::TerminalCommandStatusMessage;
pub use messages::status::TerminalCommandStatusMessageStabby;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(TerminalCommandMessageConverter, TerminalCommandMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(TerminalCommandMessageStabbyConverter, TerminalCommandMessageStabby, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(TerminalCommandStatusMessageConverter, TerminalCommandStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    TerminalCommandStatusMessageStabbyConverter,
    TerminalCommandStatusMessageStabby,
    |json: serde_json::Value| serde_json::from_value(json).unwrap_or_default()
);

/// Register all JSON converter implementations for terminal-command messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    TerminalCommandMessageConverter::register_in_host(context);
    TerminalCommandStatusMessageConverter::register_in_host(context);
}
