pub(crate) mod atomic;
pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub(crate) mod mcp;
pub(crate) mod personalization;
pub(crate) mod widget;

use crate::atomic::AudioAtomicWidget;
use crate::widget::AudioWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "audio" => audio_widget => AudioWidget => html,
    "audio_volume" => audio_volume_widget => AudioAtomicWidget,
    "audio_volume_up" => audio_volume_up_widget => AudioAtomicWidget,
    "audio_volume_down" => audio_volume_down_widget => AudioAtomicWidget,
    "audio_mute" => audio_mute_widget => AudioAtomicWidget,
    "audio_rotate_device" => audio_rotate_device_widget => AudioAtomicWidget,
    "audio_volume_span" => audio_volume_span_widget => AudioAtomicWidget,
}
