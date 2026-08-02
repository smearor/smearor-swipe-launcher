use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandWindowIdentifier;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Passes a key press to the specified window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PassDispatchMessage {
    pub window_identifier: HyprlandWindowIdentifier,
}

impl TypedMessage for PassDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::PassDispatchMessage");
}

impl MessageTopic for PassDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for PassDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
