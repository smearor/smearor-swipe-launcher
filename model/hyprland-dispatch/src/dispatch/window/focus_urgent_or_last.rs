use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Focuses the urgent window or the last focused window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FocusUrgentOrLastDispatchMessage;

impl TypedMessage for FocusUrgentOrLastDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusUrgentOrLastDispatchMessage");
}

impl MessageTopic for FocusUrgentOrLastDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusUrgentOrLastDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
