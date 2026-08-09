use crate::service::MprisService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_mpris_model::MprisSeekArgs;
use smearor_mpris_model::MprisSetPositionArgs;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for MprisService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let status_resource = RegisterResourceMessage::new(
            "mpris://status",
            "MPRIS Status",
            "Active players, playback status, track metadata, position, loop mode, shuffle, and volume.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(status_resource);

        let players_resource = RegisterResourceMessage::new(
            "mpris://players",
            "MPRIS Players",
            "List of all available MPRIS players with bus name, display name, and active flag.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(players_resource);

        let playback_resource = RegisterResourceMessage::new(
            "mpris://playback",
            "MPRIS Playback Status",
            "Current playback status (Playing, Paused, Stopped) of the active player.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(playback_resource);

        let metadata_resource = RegisterResourceMessage::new(
            "mpris://metadata",
            "MPRIS Track Metadata",
            "Metadata of the currently playing track (title, artist, album, length, art URL).",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(metadata_resource);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();

        let play_tool = RegisterToolMessage::new(
            "mpris_play",
            "Starts or resumes playback on the active MPRIS player. German synonyms: starten, abspielen, wiedergeben, Musik starten, Lied starten, Song starten, Wiedergabe starten, weitermachen, weiter abspielen.",
            &no_args_schema,
        )
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(play_tool);

        let pause_tool = RegisterToolMessage::new("mpris_pause", "Pauses playback on the active MPRIS player.", &no_args_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(pause_tool);

        let toggle_play_pause_tool =
            RegisterToolMessage::new("mpris_toggle_play_pause", "Toggles between play and pause on the active MPRIS player.", &no_args_schema)
                .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(toggle_play_pause_tool);

        let stop_tool = RegisterToolMessage::new("mpris_stop", "Stops playback on the active MPRIS player.", &no_args_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(stop_tool);

        let next_track_tool = RegisterToolMessage::new("mpris_next_track", "Skips to the next track on the active MPRIS player.", &no_args_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(next_track_tool);

        let previous_track_tool =
            RegisterToolMessage::new("mpris_previous_track", "Returns to the previous track on the active MPRIS player.", &no_args_schema)
                .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(previous_track_tool);

        let seek_schema = serde_json::to_string(&schema_for!(MprisSeekArgs)).unwrap_or_default();
        let seek_tool = RegisterToolMessage::new(
            "mpris_seek",
            "Seeks forward or backward by an offset in microseconds on the active MPRIS player.",
            &seek_schema,
        )
        .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(seek_tool);

        let set_position_schema = serde_json::to_string(&schema_for!(MprisSetPositionArgs)).unwrap_or_default();
        let set_position_tool = RegisterToolMessage::new(
            "mpris_set_position",
            "Sets the playback position to an absolute value in microseconds on the active MPRIS player.",
            &set_position_schema,
        )
        .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(set_position_tool);

        let cycle_loop_tool = RegisterToolMessage::new(
            "mpris_cycle_loop",
            "Cycles through loop modes: None -> Track -> Playlist on the active MPRIS player.",
            &no_args_schema,
        )
        .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(cycle_loop_tool);

        let toggle_shuffle_tool = RegisterToolMessage::new("mpris_toggle_shuffle", "Toggles shuffle on/off on the active MPRIS player.", &no_args_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(toggle_shuffle_tool);

        let next_player_tool = RegisterToolMessage::new("mpris_next_player", "Switches to the next available MPRIS player.", &no_args_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(next_player_tool);

        let previous_player_tool = RegisterToolMessage::new("mpris_previous_player", "Switches to the previous available MPRIS player.", &no_args_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(previous_player_tool);

        let raise_tool = RegisterToolMessage::new("mpris_raise", "Brings the active MPRIS player window to the foreground.", &no_args_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(raise_tool);

        let quit_tool = RegisterToolMessage::new("mpris_quit", "Quits the active MPRIS player application.", &no_args_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(quit_tool);

        let refresh_tool = RegisterToolMessage::new("mpris_refresh_status", "Force an immediate refresh of the MPRIS status from D-Bus.", &no_args_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(refresh_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "mpris_control_guide",
            "System message listing available players and control tool instructions.",
            &no_args_schema,
            "media player preferences and default music player",
            "player,media,music,mpris",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
