use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Adds a new master to the layout.
#[derive(Clone, Debug, Default)]
pub struct AddMasterDispatchMessage;

/// ABI-stable version of `AddMasterDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct AddMasterDispatchMessageStabby;

impl From<AddMasterDispatchMessage> for AddMasterDispatchMessageStabby {
    fn from(_value: AddMasterDispatchMessage) -> Self {
        Self
    }
}

impl From<AddMasterDispatchMessageStabby> for AddMasterDispatchMessage {
    fn from(_value: AddMasterDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for AddMasterDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::AddMasterDispatchMessage");
}

impl TypedMessage for AddMasterDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::AddMasterDispatchMessageStabby");
}

impl MessageTopic for AddMasterDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for AddMasterDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for AddMasterDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
