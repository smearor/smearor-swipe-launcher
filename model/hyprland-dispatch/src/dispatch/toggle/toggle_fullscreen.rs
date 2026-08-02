use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandFullscreenType;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Toggles fullscreen for the active window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToggleFullscreenDispatchMessage {
    pub fullscreen_type: HyprlandFullscreenType,
}

impl TypedMessage for ToggleFullscreenDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleFullscreenDispatchMessage");
}

impl MessageTopic for ToggleFullscreenDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleFullscreenDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
