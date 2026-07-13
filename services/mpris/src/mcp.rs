use crate::service::MprisService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::debug;

impl MprisService {
    /// Registers all MCP resources and tools exposed by the MPRIS service.
    pub fn register_mcp_capabilities(&self) {
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

        let play_tool = RegisterToolMessage::new(
            "mpris_play",
            "Starts or resumes playback on the active MPRIS player.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(play_tool);

        let pause_tool = RegisterToolMessage::new("mpris_pause", "Pauses playback on the active MPRIS player.", r#"{ "type": "object", "properties": {} }"#);
        broadcaster.broadcast_message_to_topic(pause_tool);

        let toggle_play_pause_tool = RegisterToolMessage::new(
            "mpris_toggle_play_pause",
            "Toggles between play and pause on the active MPRIS player.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(toggle_play_pause_tool);

        let stop_tool = RegisterToolMessage::new("mpris_stop", "Stops playback on the active MPRIS player.", r#"{ "type": "object", "properties": {} }"#);
        broadcaster.broadcast_message_to_topic(stop_tool);

        let next_track_tool = RegisterToolMessage::new(
            "mpris_next_track",
            "Skips to the next track on the active MPRIS player.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(next_track_tool);

        let previous_track_tool = RegisterToolMessage::new(
            "mpris_previous_track",
            "Returns to the previous track on the active MPRIS player.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(previous_track_tool);

        let seek_tool = RegisterToolMessage::new(
            "mpris_seek",
            "Seeks forward or backward by an offset in microseconds on the active MPRIS player.",
            r#"{ "type": "object", "properties": { "offset": { "type": "integer", "description": "Seek offset in microseconds (positive or negative)" } }, "required": ["offset"] }"#,
        );
        broadcaster.broadcast_message_to_topic(seek_tool);

        let set_position_tool = RegisterToolMessage::new(
            "mpris_set_position",
            "Sets the playback position to an absolute value in microseconds on the active MPRIS player.",
            r#"{ "type": "object", "properties": { "position": { "type": "integer", "description": "Absolute position in microseconds" } }, "required": ["position"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_position_tool);

        let cycle_loop_tool = RegisterToolMessage::new(
            "mpris_cycle_loop",
            "Cycles through loop modes: None -> Track -> Playlist on the active MPRIS player.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(cycle_loop_tool);

        let toggle_shuffle_tool = RegisterToolMessage::new(
            "mpris_toggle_shuffle",
            "Toggles shuffle on/off on the active MPRIS player.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(toggle_shuffle_tool);

        let next_player_tool = RegisterToolMessage::new(
            "mpris_next_player",
            "Switches to the next available MPRIS player.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(next_player_tool);

        let previous_player_tool = RegisterToolMessage::new(
            "mpris_previous_player",
            "Switches to the previous available MPRIS player.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(previous_player_tool);

        let raise_tool = RegisterToolMessage::new(
            "mpris_raise",
            "Brings the active MPRIS player window to the foreground.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(raise_tool);

        let quit_tool = RegisterToolMessage::new("mpris_quit", "Quits the active MPRIS player application.", r#"{ "type": "object", "properties": {} }"#);
        broadcaster.broadcast_message_to_topic(quit_tool);

        let refresh_tool = RegisterToolMessage::new(
            "mpris_refresh_status",
            "Force an immediate refresh of the MPRIS status from D-Bus.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(refresh_tool);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for MprisService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("MPRIS Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();

        match tool_name.as_str() {
            "mpris_play" => {
                self.handle_play();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Playback started");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_pause" => {
                self.handle_pause();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Playback paused");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_toggle_play_pause" => {
                self.handle_toggle_play_pause();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Play/pause toggled");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_stop" => {
                self.handle_stop();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Playback stopped");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_next_track" => {
                self.handle_next_track();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Skipped to next track");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_previous_track" => {
                self.handle_previous_track();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Returned to previous track");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_seek" => {
                let offset = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string())
                    .ok()
                    .and_then(|v| v.get("offset").and_then(|a| a.as_i64()))
                    .unwrap_or(0);
                self.handle_seek(offset);
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Seeked by {offset} microseconds"));
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_set_position" => {
                let position = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string())
                    .ok()
                    .and_then(|v| v.get("position").and_then(|a| a.as_i64()))
                    .unwrap_or(0);
                self.handle_set_position(position);
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Position set to {position} microseconds"));
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_cycle_loop" => {
                self.handle_cycle_loop();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Loop mode cycled");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_toggle_shuffle" => {
                self.handle_toggle_shuffle();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Shuffle toggled");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_next_player" => {
                self.handle_next_player();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Switched to next player");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_previous_player" => {
                self.handle_previous_player();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Switched to previous player");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_raise" => {
                self.handle_raise();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Player window raised");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_quit" => {
                self.handle_quit();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Player application quit");
                broadcaster.broadcast_message_to_topic(response);
            }
            "mpris_refresh_status" => {
                let _ = self.command_sender.send(crate::mpris_command::MprisCommand::RefreshStatus);
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Status refresh triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
            _ => {
                let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Unknown tool: {tool_name}"));
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for MprisService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        let uri = message.0.uri.to_string();
        debug!("MPRIS Service: InvokeResourceMessage uri={}", uri);
        let broadcaster = self.get_broadcaster();

        let status = self.status_snapshot();
        let response = match uri.as_str() {
            "mpris://status" => match &status {
                Some(s) => {
                    let json = serde_json::json!({
                        "has_player": s.has_player,
                        "active_player": s.active_player.as_ref().map(|p| serde_json::json!({
                            "bus_name": p.bus_name.to_string(),
                            "name": p.name.to_string(),
                            "is_active": p.is_active,
                        })).unwrap_or(serde_json::Value::Null),
                        "players": s.players.iter().map(|p| serde_json::json!({
                            "bus_name": p.bus_name.to_string(),
                            "name": p.name.to_string(),
                            "is_active": p.is_active,
                        })).collect::<Vec<_>>(),
                        "playback_status": format!("{:?}", s.playback_status),
                        "metadata": s.metadata.as_ref().map(|m| serde_json::json!({
                            "title": m.title.to_string(),
                            "artist": m.artist.to_string(),
                            "album": m.album.to_string(),
                            "length": m.length,
                            "art_url": m.art_url.as_ref().map(|a| serde_json::Value::String(a.to_string())).unwrap_or(serde_json::Value::Null),
                        })).unwrap_or(serde_json::Value::Null),
                        "position": s.position,
                        "loop_status": format!("{:?}", s.loop_status),
                        "shuffle": s.shuffle,
                        "volume": s.volume,
                    });
                    InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(&message.0.correlation_id, "MPRIS status not yet available"),
            },
            "mpris://players" => match &status {
                Some(s) => {
                    let players: Vec<serde_json::Value> = s
                        .players
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "bus_name": p.bus_name.to_string(),
                                "name": p.name.to_string(),
                                "is_active": p.is_active,
                            })
                        })
                        .collect();
                    let json = serde_json::Value::Array(players);
                    InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(&message.0.correlation_id, "MPRIS status not yet available"),
            },
            "mpris://playback" => match &status {
                Some(s) => {
                    let json = serde_json::json!({
                        "has_player": s.has_player,
                        "playback_status": format!("{:?}", s.playback_status),
                    });
                    InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(&message.0.correlation_id, "MPRIS status not yet available"),
            },
            "mpris://metadata" => match &status {
                Some(s) => match s.metadata.as_ref() {
                    Some(m) => {
                        let json = serde_json::json!({
                            "title": m.title.to_string(),
                            "artist": m.artist.to_string(),
                            "album": m.album.to_string(),
                            "length": m.length,
                            "art_url": m.art_url.as_ref().map(|a| serde_json::Value::String(a.to_string())).unwrap_or(serde_json::Value::Null),
                        });
                        InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                    }
                    None => InvokeResourceResponse::success(&message.0.correlation_id, "null"),
                },
                None => InvokeResourceResponse::error(&message.0.correlation_id, "MPRIS status not yet available"),
            },
            _ => InvokeResourceResponse::error(&message.0.correlation_id, &format!("Unknown resource: {uri}")),
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
