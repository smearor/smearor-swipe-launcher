#![recursion_limit = "512"]

pub mod dispatch;
pub mod dispatch_message;

pub use dispatch::*;
pub use dispatch_message::HyprlandSystemDispatchMessage;
pub use dispatch_message::HyprlandToggleDispatchMessage;
pub use dispatch_message::HyprlandWindowDispatchMessage;
pub use dispatch_message::HyprlandWorkspaceDispatchMessage;
pub use dispatch_message::TOPIC_SYSTEM_DISPATCH;
pub use dispatch_message::TOPIC_TOGGLE_DISPATCH;
pub use dispatch_message::TOPIC_WINDOW_DISPATCH;
pub use dispatch_message::TOPIC_WORKSPACE_DISPATCH;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

impl_json_convertible!(HyprlandWindowDispatchMessageConverter, HyprlandWindowDispatchMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandWorkspaceDispatchMessageConverter, HyprlandWorkspaceDispatchMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandToggleDispatchMessageConverter, HyprlandToggleDispatchMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandSystemDispatchMessageConverter, HyprlandSystemDispatchMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register JSON converters for dispatch messages.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    HyprlandWindowDispatchMessageConverter::register_in_host(context);
    HyprlandWorkspaceDispatchMessageConverter::register_in_host(context);
    HyprlandToggleDispatchMessageConverter::register_in_host(context);
    HyprlandSystemDispatchMessageConverter::register_in_host(context);
}
