use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Forces the renderer to reload.
#[derive(Clone, Debug, Default)]
pub struct ForceRendererReloadDispatchMessage;

/// ABI-stable version of `ForceRendererReloadDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ForceRendererReloadDispatchMessageStabby;

impl From<ForceRendererReloadDispatchMessage> for ForceRendererReloadDispatchMessageStabby {
    fn from(_value: ForceRendererReloadDispatchMessage) -> Self {
        Self
    }
}

impl From<ForceRendererReloadDispatchMessageStabby> for ForceRendererReloadDispatchMessage {
    fn from(_value: ForceRendererReloadDispatchMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for ForceRendererReloadDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ForceRendererReloadDispatchMessage");
}

impl TypedMessage for ForceRendererReloadDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ForceRendererReloadDispatchMessageStabby");
}

impl MessageTopic for ForceRendererReloadDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ForceRendererReloadDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ForceRendererReloadDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
