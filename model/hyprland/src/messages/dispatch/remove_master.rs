use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Removes a master from the layout.
#[derive(Clone, Debug, Default)]
pub struct RemoveMasterDispatchMessage;

/// ABI-stable version of `RemoveMasterDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct RemoveMasterDispatchMessageStabby;

impl From<RemoveMasterDispatchMessage> for RemoveMasterDispatchMessageStabby {
    fn from(_value: RemoveMasterDispatchMessage) -> Self {
        Self
    }
}

impl From<RemoveMasterDispatchMessageStabby> for RemoveMasterDispatchMessage {
    fn from(_value: RemoveMasterDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for RemoveMasterDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::RemoveMasterDispatchMessage");
}

impl TypedMessage for RemoveMasterDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::RemoveMasterDispatchMessageStabby");
}

impl MessageTopic for RemoveMasterDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for RemoveMasterDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for RemoveMasterDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
