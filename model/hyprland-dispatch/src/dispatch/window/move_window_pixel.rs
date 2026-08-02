use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandPosition;
use smearor_hyprland_shared::HyprlandWindowIdentifier;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves a specific window by pixel position delta or to an exact position.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveWindowPixelDispatchMessage {
    pub position: HyprlandPosition,
    pub window_identifier: HyprlandWindowIdentifier,
}

impl TypedMessage for MoveWindowPixelDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveWindowPixelDispatchMessage");
}

impl MessageTopic for MoveWindowPixelDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveWindowPixelDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
