use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Focuses the current window or the last focused window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FocusCurrentOrLastDispatchMessage;

impl TypedMessage for FocusCurrentOrLastDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusCurrentOrLastDispatchMessage");
}

impl MessageTopic for FocusCurrentOrLastDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusCurrentOrLastDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
