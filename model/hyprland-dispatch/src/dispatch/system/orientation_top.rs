use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Sets the orientation of the active window to top.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OrientationTopDispatchMessage;

impl TypedMessage for OrientationTopDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OrientationTopDispatchMessage");
}

impl MessageTopic for OrientationTopDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for OrientationTopDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
