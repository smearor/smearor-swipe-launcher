use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Toggles fake fullscreen state for the active window.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToggleFakeFullscreenDispatchMessage;

impl TypedMessage for ToggleFakeFullscreenDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleFakeFullscreenDispatchMessage");
}

impl MessageTopic for ToggleFakeFullscreenDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleFakeFullscreenDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
