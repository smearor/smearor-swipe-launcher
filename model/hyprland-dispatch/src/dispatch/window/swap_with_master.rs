use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandSwapWithMasterParam;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Swaps the active window with the master or a child.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SwapWithMasterDispatchMessage {
    pub param: HyprlandSwapWithMasterParam,
}

impl TypedMessage for SwapWithMasterDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwapWithMasterDispatchMessage");
}

impl MessageTopic for SwapWithMasterDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for SwapWithMasterDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
