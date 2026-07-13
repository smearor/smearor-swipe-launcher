pub mod config;
pub mod views;
pub mod widget;

use crate::widget::VoiceAssistantWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin;

widget_plugin!(VoiceAssistantWidget);
