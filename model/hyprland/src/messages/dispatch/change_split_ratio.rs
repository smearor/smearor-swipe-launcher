use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Changes the split ratio of the active window.
#[derive(Clone, Debug, Default)]
pub struct ChangeSplitRatioDispatchMessage {
    pub ratio: f32,
}

/// ABI-stable version of `ChangeSplitRatioDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ChangeSplitRatioDispatchMessageStabby {
    pub ratio: f32,
}

impl From<ChangeSplitRatioDispatchMessage> for ChangeSplitRatioDispatchMessageStabby {
    fn from(value: ChangeSplitRatioDispatchMessage) -> Self {
        Self { ratio: value.ratio }
    }
}

impl From<ChangeSplitRatioDispatchMessageStabby> for ChangeSplitRatioDispatchMessage {
    fn from(value: ChangeSplitRatioDispatchMessageStabby) -> Self {
        Self { ratio: value.ratio }
    }
}

impl TypedMessage for ChangeSplitRatioDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ChangeSplitRatioDispatchMessage");
}

impl TypedMessage for ChangeSplitRatioDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ChangeSplitRatioDispatchMessageStabby");
}

impl MessageTopic for ChangeSplitRatioDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ChangeSplitRatioDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ChangeSplitRatioDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
