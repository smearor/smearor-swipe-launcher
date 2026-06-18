use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use stabby::option::Option as StabbyOption;
use stabby::vec::Vec as StabbyVec;

use crate::MprisCommandAction;
use crate::MprisCommandMessage;
use crate::MprisLoopStatus;
use crate::MprisPlaybackStatus;
use crate::MprisPlayerInfo;
use crate::MprisStatusMessage;
use crate::MprisTrackMetadata;

fn parse_mpris_command_action(value: &serde_json::Value) -> MprisCommandAction {
    match value.as_str() {
        Some("Pause") => MprisCommandAction::Pause,
        Some("TogglePlayPause") => MprisCommandAction::TogglePlayPause,
        Some("Stop") => MprisCommandAction::Stop,
        Some("NextTrack") => MprisCommandAction::NextTrack,
        Some("PreviousTrack") => MprisCommandAction::PreviousTrack,
        Some("Seek") => MprisCommandAction::Seek,
        Some("SetPosition") => MprisCommandAction::SetPosition,
        Some("CycleLoop") => MprisCommandAction::CycleLoop,
        Some("ToggleShuffle") => MprisCommandAction::ToggleShuffle,
        Some("NextPlayer") => MprisCommandAction::NextPlayer,
        Some("PreviousPlayer") => MprisCommandAction::PreviousPlayer,
        Some("Raise") => MprisCommandAction::Raise,
        Some("Quit") => MprisCommandAction::Quit,
        _ => MprisCommandAction::Play,
    }
}

fn parse_mpris_playback_status(value: &serde_json::Value) -> MprisPlaybackStatus {
    match value.as_str() {
        Some("Paused") => MprisPlaybackStatus::Paused,
        Some("Stopped") => MprisPlaybackStatus::Stopped,
        _ => MprisPlaybackStatus::Playing,
    }
}

fn parse_mpris_loop_status(value: &serde_json::Value) -> MprisLoopStatus {
    match value.as_str() {
        Some("Track") => MprisLoopStatus::Track,
        Some("Playlist") => MprisLoopStatus::Playlist,
        _ => MprisLoopStatus::None,
    }
}

fn parse_mpris_player_info(value: &serde_json::Value) -> MprisPlayerInfo {
    MprisPlayerInfo {
        bus_name: value.get("bus_name").and_then(|v| v.as_str()).unwrap_or("").into(),
        name: value.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
        is_active: value.get("is_active").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

fn parse_mpris_track_metadata(value: &serde_json::Value) -> MprisTrackMetadata {
    MprisTrackMetadata {
        title: value.get("title").and_then(|v| v.as_str()).unwrap_or("").into(),
        artist: value.get("artist").and_then(|v| v.as_str()).unwrap_or("").into(),
        album: value.get("album").and_then(|v| v.as_str()).unwrap_or("").into(),
        length: value.get("length").and_then(|v| v.as_i64()).unwrap_or(0),
        art_url: value
            .get("art_url")
            .and_then(|v| if v.is_null() { None } else { Some(v.as_str().unwrap_or("").into()) })
            .map(StabbyOption::Some)
            .unwrap_or(StabbyOption::None()),
    }
}

fn parse_mpris_players(value: &serde_json::Value) -> StabbyVec<MprisPlayerInfo> {
    let mut players = StabbyVec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            players.push(parse_mpris_player_info(item));
        }
    }
    players
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MprisCommandMessageConverter, MprisCommandMessage, |json: serde_json::Value| {
    let action = parse_mpris_command_action(json.get("action").unwrap_or(&serde_json::Value::Null));
    let seek_offset = json
        .get("seek_offset")
        .and_then(|v| v.as_i64())
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());
    let position = json
        .get("position")
        .and_then(|v| v.as_i64())
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());
    let player_bus_name = json
        .get("player_bus_name")
        .and_then(|v| if v.is_null() { None } else { Some(v.as_str().unwrap_or("").into()) })
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());
    MprisCommandMessage::new(action, seek_offset, position, player_bus_name)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(MprisStatusMessageConverter, MprisStatusMessage, |json: serde_json::Value| {
    let has_player = json.get("has_player").and_then(|v| v.as_bool()).unwrap_or(false);
    let active_player = json
        .get("active_player")
        .and_then(|v| if v.is_null() { None } else { Some(parse_mpris_player_info(v)) })
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());
    let players = parse_mpris_players(json.get("players").unwrap_or(&serde_json::Value::Null));
    let playback_status = parse_mpris_playback_status(json.get("playback_status").unwrap_or(&serde_json::Value::Null));
    let metadata = json
        .get("metadata")
        .and_then(|v| if v.is_null() { None } else { Some(parse_mpris_track_metadata(v)) })
        .map(StabbyOption::Some)
        .unwrap_or(StabbyOption::None());
    let position = json.get("position").and_then(|v| v.as_i64()).unwrap_or(0);
    let loop_status = parse_mpris_loop_status(json.get("loop_status").unwrap_or(&serde_json::Value::Null));
    let shuffle = json.get("shuffle").and_then(|v| v.as_bool()).unwrap_or(false);
    let volume = json.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    MprisStatusMessage::new(has_player, active_player, players, playback_status, metadata, position, loop_status, shuffle, volume)
});

/// Register all JSON converter implementations for MPRIS messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    MprisCommandMessageConverter::register_in_host(context);
    MprisStatusMessageConverter::register_in_host(context);
}
