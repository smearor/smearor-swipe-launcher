use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandDirection;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves focus in the given direction.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveFocusDispatchMessage {
    pub direction: HyprlandDirection,
}

impl TypedMessage for MoveFocusDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveFocusDispatchMessage");
}

impl MessageTopic for MoveFocusDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveFocusDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
