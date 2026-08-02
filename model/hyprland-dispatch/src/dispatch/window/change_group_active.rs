use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandWindowSwitchDirection;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Changes the active window in a group.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChangeGroupActiveDispatchMessage {
    pub direction: HyprlandWindowSwitchDirection,
}

impl TypedMessage for ChangeGroupActiveDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ChangeGroupActiveDispatchMessage");
}

impl MessageTopic for ChangeGroupActiveDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ChangeGroupActiveDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
