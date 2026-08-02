use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Removes a master from the layout.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RemoveMasterDispatchMessage;

impl TypedMessage for RemoveMasterDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::RemoveMasterDispatchMessage");
}

impl MessageTopic for RemoveMasterDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for RemoveMasterDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
