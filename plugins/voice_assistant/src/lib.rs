pub(crate) mod atomic;
pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub(crate) mod personalization;
pub(crate) mod views;
pub(crate) mod widget;

use crate::atomic::VoiceAssistantAtomicWidget;
use crate::widget::VoiceAssistantWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "voice_assistant" => voice_assistant_widget => VoiceAssistantWidget => html,
    "voice_assistant_listen" => voice_assistant_listen_widget => VoiceAssistantAtomicWidget,
    "voice_assistant_push_to_talk" => voice_assistant_push_to_talk_widget => VoiceAssistantAtomicWidget,
    "voice_assistant_stop" => voice_assistant_stop_widget => VoiceAssistantAtomicWidget,
    "voice_assistant_status" => voice_assistant_status_widget => VoiceAssistantAtomicWidget,
}
