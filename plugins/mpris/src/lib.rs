pub(crate) mod atomic;
pub(crate) mod atomic_graphic;
pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub(crate) mod mcp;
pub(crate) mod personalization;
pub(crate) mod widget;

use crate::atomic::MprisAtomicWidget;
use crate::widget::MprisWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "mpris" => mpris_widget => MprisWidget => html,
    "mpris_song" => mpris_song_widget => MprisAtomicWidget,
    "mpris_artist" => mpris_artist_widget => MprisAtomicWidget,
    "mpris_album" => mpris_album_widget => MprisAtomicWidget,
    "mpris_next" => mpris_next_widget => MprisAtomicWidget,
    "mpris_previous" => mpris_previous_widget => MprisAtomicWidget,
    "mpris_play_pause" => mpris_play_pause_widget => MprisAtomicWidget,
    "mpris_stop" => mpris_stop_widget => MprisAtomicWidget,
    "mpris_switch_player" => mpris_switch_player_widget => MprisAtomicWidget,
    "mpris_seek_forward" => mpris_seek_forward_widget => MprisAtomicWidget,
    "mpris_seek_backward" => mpris_seek_backward_widget => MprisAtomicWidget,
    "mpris_shuffle" => mpris_shuffle_widget => MprisAtomicWidget,
    "mpris_repeat" => mpris_repeat_widget => MprisAtomicWidget,
}
