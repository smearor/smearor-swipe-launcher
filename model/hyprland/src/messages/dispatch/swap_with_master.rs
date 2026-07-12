use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandSwapWithMasterParam;

use super::workspace::TOPIC_DISPATCH;

/// Swaps the active window with the master or a child.
#[derive(Clone, Debug, Default)]
pub struct SwapWithMasterDispatchMessage {
    pub param: HyprlandSwapWithMasterParam,
}

/// ABI-stable version of `SwapWithMasterDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SwapWithMasterDispatchMessageStabby {
    pub param: HyprlandSwapWithMasterParam,
}

impl From<SwapWithMasterDispatchMessage> for SwapWithMasterDispatchMessageStabby {
    fn from(value: SwapWithMasterDispatchMessage) -> Self {
        Self { param: value.param }
    }
}

impl From<SwapWithMasterDispatchMessageStabby> for SwapWithMasterDispatchMessage {
    fn from(value: SwapWithMasterDispatchMessageStabby) -> Self {
        Self { param: value.param }
    }
}

impl TypedMessage for SwapWithMasterDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwapWithMasterDispatchMessage");
}

impl TypedMessage for SwapWithMasterDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwapWithMasterDispatchMessageStabby");
}

impl MessageTopic for SwapWithMasterDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for SwapWithMasterDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for SwapWithMasterDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
