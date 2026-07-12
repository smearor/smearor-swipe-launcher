use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandWindowSwitchDirection;

use super::workspace::TOPIC_DISPATCH;

/// Changes the active window in a group.
#[derive(Clone, Debug, Default)]
pub struct ChangeGroupActiveDispatchMessage {
    pub direction: HyprlandWindowSwitchDirection,
}

/// ABI-stable version of `ChangeGroupActiveDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ChangeGroupActiveDispatchMessageStabby {
    pub direction: HyprlandWindowSwitchDirection,
}

impl From<ChangeGroupActiveDispatchMessage> for ChangeGroupActiveDispatchMessageStabby {
    fn from(value: ChangeGroupActiveDispatchMessage) -> Self {
        Self { direction: value.direction }
    }
}

impl From<ChangeGroupActiveDispatchMessageStabby> for ChangeGroupActiveDispatchMessage {
    fn from(value: ChangeGroupActiveDispatchMessageStabby) -> Self {
        Self { direction: value.direction }
    }
}

impl TypedMessage for ChangeGroupActiveDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ChangeGroupActiveDispatchMessage");
}

impl TypedMessage for ChangeGroupActiveDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ChangeGroupActiveDispatchMessageStabby");
}

impl MessageTopic for ChangeGroupActiveDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ChangeGroupActiveDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ChangeGroupActiveDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
