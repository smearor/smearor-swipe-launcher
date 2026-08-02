use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandPosition;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves the active window by the given position delta or to an exact position.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveActiveDispatchMessage {
    pub position: HyprlandPosition,
}

impl TypedMessage for MoveActiveDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveActiveDispatchMessage");
}

impl MessageTopic for MoveActiveDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveActiveDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
