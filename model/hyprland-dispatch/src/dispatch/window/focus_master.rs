use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandFocusMasterParam;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Focuses the master window or auto-selects.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FocusMasterDispatchMessage {
    pub param: HyprlandFocusMasterParam,
}

impl TypedMessage for FocusMasterDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusMasterDispatchMessage");
}

impl MessageTopic for FocusMasterDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusMasterDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
