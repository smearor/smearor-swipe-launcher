use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Changes the split ratio of the active window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChangeSplitRatioDispatchMessage {
    pub ratio: f32,
}

impl TypedMessage for ChangeSplitRatioDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ChangeSplitRatioDispatchMessage");
}

impl MessageTopic for ChangeSplitRatioDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ChangeSplitRatioDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
