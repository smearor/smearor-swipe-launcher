use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandWindowMove;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves the active window in the specified direction or to a monitor.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveWindowDispatchMessage {
    pub window_move: HyprlandWindowMove,
}

impl TypedMessage for MoveWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveWindowDispatchMessage");
}

impl MessageTopic for MoveWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveWindowDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
