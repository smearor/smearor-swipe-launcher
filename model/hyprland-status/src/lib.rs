#![recursion_limit = "512"]
#![allow(long_running_const_eval)]

pub mod state_request;
pub mod status;
pub mod status_event;

pub use state_request::HyprlandStateMessage;
pub use state_request::HyprlandStateRequestMessage;
pub use state_request::TOPIC_HYPRLAND_STATE;
pub use state_request::TOPIC_HYPRLAND_STATE_REQUEST;
pub use status::*;
pub use status_event::HyprlandGroupStatusMessage;
pub use status_event::HyprlandLayerStatusMessage;
pub use status_event::HyprlandSystemStatusMessage;
pub use status_event::HyprlandWindowStatusMessage;
pub use status_event::HyprlandWorkspaceStatusMessage;
pub use status_event::TOPIC_HYPRLAND_GROUP_STATUS;
pub use status_event::TOPIC_HYPRLAND_LAYER_STATUS;
pub use status_event::TOPIC_HYPRLAND_SYSTEM_STATUS;
pub use status_event::TOPIC_HYPRLAND_WINDOW_STATUS;
pub use status_event::TOPIC_HYPRLAND_WORKSPACE_STATUS;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

impl_json_convertible!(HyprlandWindowStatusMessageConverter, HyprlandWindowStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandWorkspaceStatusMessageConverter, HyprlandWorkspaceStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandGroupStatusMessageConverter, HyprlandGroupStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandLayerStatusMessageConverter, HyprlandLayerStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandSystemStatusMessageConverter, HyprlandSystemStatusMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandStateRequestConverter, HyprlandStateRequestMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(HyprlandStateConverter, HyprlandStateMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});

/// Register JSON converters for status and state messages.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    HyprlandWindowStatusMessageConverter::register_in_host(context);
    HyprlandWorkspaceStatusMessageConverter::register_in_host(context);
    HyprlandGroupStatusMessageConverter::register_in_host(context);
    HyprlandLayerStatusMessageConverter::register_in_host(context);
    HyprlandSystemStatusMessageConverter::register_in_host(context);
    HyprlandStateRequestConverter::register_in_host(context);
    HyprlandStateConverter::register_in_host(context);
}
