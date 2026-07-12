use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Brings the active window to the top of the z-order.
#[derive(Clone, Debug, Default)]
pub struct BringActiveToTopDispatchMessage;

/// ABI-stable version of `BringActiveToTopDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct BringActiveToTopDispatchMessageStabby;

impl From<BringActiveToTopDispatchMessage> for BringActiveToTopDispatchMessageStabby {
    fn from(_value: BringActiveToTopDispatchMessage) -> Self {
        Self
    }
}

impl From<BringActiveToTopDispatchMessageStabby> for BringActiveToTopDispatchMessage {
    fn from(_value: BringActiveToTopDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for BringActiveToTopDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::BringActiveToTopDispatchMessage");
}

impl TypedMessage for BringActiveToTopDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::BringActiveToTopDispatchMessageStabby");
}

impl MessageTopic for BringActiveToTopDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for BringActiveToTopDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for BringActiveToTopDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
