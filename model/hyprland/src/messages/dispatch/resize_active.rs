use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandPosition;

use super::workspace::TOPIC_DISPATCH;

/// Resizes the active window by the given position delta or to an exact position.
#[derive(Clone, Debug, Default)]
pub struct ResizeActiveDispatchMessage {
    pub position: HyprlandPosition,
}

/// ABI-stable version of `ResizeActiveDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ResizeActiveDispatchMessageStabby {
    pub position: HyprlandPosition,
}

impl From<ResizeActiveDispatchMessage> for ResizeActiveDispatchMessageStabby {
    fn from(value: ResizeActiveDispatchMessage) -> Self {
        Self { position: value.position }
    }
}

impl From<ResizeActiveDispatchMessageStabby> for ResizeActiveDispatchMessage {
    fn from(value: ResizeActiveDispatchMessageStabby) -> Self {
        Self { position: value.position }
    }
}

impl TypedMessage for ResizeActiveDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ResizeActiveDispatchMessage");
}

impl TypedMessage for ResizeActiveDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ResizeActiveDispatchMessageStabby");
}

impl MessageTopic for ResizeActiveDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ResizeActiveDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ResizeActiveDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
