use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::HyprlandFocusMasterParam;

use super::workspace::TOPIC_DISPATCH;

/// Focuses the master window or auto-selects.
#[derive(Clone, Debug, Default)]
pub struct FocusMasterDispatchMessage {
    pub param: HyprlandFocusMasterParam,
}

/// ABI-stable version of `FocusMasterDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct FocusMasterDispatchMessageStabby {
    pub param: HyprlandFocusMasterParam,
}

impl From<FocusMasterDispatchMessage> for FocusMasterDispatchMessageStabby {
    fn from(value: FocusMasterDispatchMessage) -> Self {
        Self { param: value.param }
    }
}

impl From<FocusMasterDispatchMessageStabby> for FocusMasterDispatchMessage {
    fn from(value: FocusMasterDispatchMessageStabby) -> Self {
        Self { param: value.param }
    }
}

impl TypedMessage for FocusMasterDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusMasterDispatchMessage");
}

impl TypedMessage for FocusMasterDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::FocusMasterDispatchMessageStabby");
}

impl MessageTopic for FocusMasterDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for FocusMasterDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for FocusMasterDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
