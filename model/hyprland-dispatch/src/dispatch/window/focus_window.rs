use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWindowIdentifier;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Focuses the specified window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FocusWindowDispatchMessage {
    pub window_identifier: HyprlandWindowIdentifier,
}

impl TypedMessage for FocusWindowDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusWindowDispatchMessage");
}

impl MessageTopic for FocusWindowDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusWindowDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
