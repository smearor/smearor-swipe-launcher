use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Brings the active window to the top of the z-order.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BringActiveToTopDispatchMessage;

impl TypedMessage for BringActiveToTopDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::BringActiveToTopDispatchMessage");
}

impl MessageTopic for BringActiveToTopDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for BringActiveToTopDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
