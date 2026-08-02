use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Toggles the group state for the active window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToggleGroupDispatchMessage;

impl TypedMessage for ToggleGroupDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleGroupDispatchMessage");
}

impl MessageTopic for ToggleGroupDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleGroupDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
