use crate::service::AudioService;
use smearor_audio_model::AudioStatusMessage;
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

impl AudioService {
    /// Registers all MCP resources and tools exposed by the audio service.
    pub fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let status_resource = RegisterResourceMessage::new(
            "audio://status",
            "Audio Status",
            "Complete audio status including volume, mute, output devices, and active device.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(status_resource);

        let volume_resource = RegisterResourceMessage::new("audio://volume", "Audio Volume", "Current master volume level (0.0 to 1.0).", "application/json");
        broadcaster.broadcast_message_to_topic(volume_resource);

        let muted_resource = RegisterResourceMessage::new("audio://muted", "Audio Muted", "Current mute status of the default sink.", "application/json");
        broadcaster.broadcast_message_to_topic(muted_resource);

        let active_sink_resource = RegisterResourceMessage::new(
            "audio://active_sink",
            "Active Audio Sink",
            "Currently active output device with name, index, and default flag.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(active_sink_resource);

        let sinks_resource = RegisterResourceMessage::new("audio://sinks", "Audio Output Devices", "List of all available output devices.", "application/json");
        broadcaster.broadcast_message_to_topic(sinks_resource);

        let volume_up_tool = RegisterToolMessage::new(
            "audio_volume_up",
            "Increases the audio volume by a configured step.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(volume_up_tool);

        let volume_down_tool = RegisterToolMessage::new(
            "audio_volume_down",
            "Decreases the audio volume by a configured step.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(volume_down_tool);

        let set_volume_tool = RegisterToolMessage::new(
            "audio_set_volume",
            "Sets the audio volume to an absolute value.",
            r#"{ "type": "object", "properties": { "volume": { "type": "number", "minimum": 0.0, "maximum": 1.0, "description": "Absolute volume level between 0.0 and 1.0" } }, "required": ["volume"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_volume_tool);

        let toggle_mute_tool = RegisterToolMessage::new(
            "audio_toggle_mute",
            "Toggles the mute state of the default sink.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(toggle_mute_tool);

        let mute_tool = RegisterToolMessage::new("audio_mute", "Mutes the default audio output sink.", r#"{ "type": "object", "properties": {} }"#);
        broadcaster.broadcast_message_to_topic(mute_tool);

        let unmute_tool = RegisterToolMessage::new("audio_unmute", "Unmutes the default audio output sink.", r#"{ "type": "object", "properties": {} }"#);
        broadcaster.broadcast_message_to_topic(unmute_tool);

        let next_device_tool = RegisterToolMessage::new(
            "audio_next_device",
            "Switches to the next available audio output device.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(next_device_tool);

        let previous_device_tool = RegisterToolMessage::new(
            "audio_previous_device",
            "Switches to the previous available audio output device.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(previous_device_tool);

        let refresh_tool = RegisterToolMessage::new(
            "audio_refresh_status",
            "Force an immediate refresh of the audio status from PulseAudio.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(refresh_tool);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Audio Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();

        match tool_name.as_str() {
            "audio_volume_up" => {
                self.handle_volume_up();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Volume increased");
                broadcaster.broadcast_message_to_topic(response);
            }
            "audio_volume_down" => {
                self.handle_volume_down();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Volume decreased");
                broadcaster.broadcast_message_to_topic(response);
            }
            "audio_set_volume" => {
                let volume = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string())
                    .ok()
                    .and_then(|v| v.get("volume").and_then(|a| a.as_f64()).map(|f| f as f32))
                    .unwrap_or(0.0);
                self.handle_set_volume(volume);
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Volume set to {volume}"));
                broadcaster.broadcast_message_to_topic(response);
            }
            "audio_toggle_mute" => {
                self.handle_toggle_mute();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Mute toggled");
                broadcaster.broadcast_message_to_topic(response);
            }
            "audio_mute" => {
                self.handle_mute();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Audio muted");
                broadcaster.broadcast_message_to_topic(response);
            }
            "audio_unmute" => {
                self.handle_unmute();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Audio unmuted");
                broadcaster.broadcast_message_to_topic(response);
            }
            "audio_next_device" => {
                self.handle_next_device();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Switched to next device");
                broadcaster.broadcast_message_to_topic(response);
            }
            "audio_previous_device" => {
                self.handle_previous_device();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Switched to previous device");
                broadcaster.broadcast_message_to_topic(response);
            }
            "audio_refresh_status" => {
                self.handle_refresh_status();
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

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        let uri = message.0.uri.to_string();
        debug!("Audio Service: InvokeResourceMessage uri={}", uri);
        let broadcaster = self.get_broadcaster();

        let status = self.status_snapshot();
        let response = match uri.as_str() {
            "audio://status" => match &status {
                Some(s) => {
                    let json = serde_json::json!({
                        "volume": s.volume,
                        "is_muted": s.is_muted,
                        "active_device": s.active_device.as_ref().map(|d| serde_json::json!({
                            "id": d.id,
                            "name": d.name.to_string(),
                            "is_default": d.is_default,
                        })).unwrap_or(serde_json::Value::Null),
                        "output_devices": s.output_devices.iter().map(|d| serde_json::json!({
                            "id": d.id,
                            "name": d.name.to_string(),
                            "is_default": d.is_default,
                        })).collect::<Vec<_>>(),
                    });
                    InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(&message.0.correlation_id, "Audio status not yet available"),
            },
            "audio://volume" => match &status {
                Some(s) => {
                    let json = serde_json::json!({ "volume": s.volume });
                    InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(&message.0.correlation_id, "Audio status not yet available"),
            },
            "audio://muted" => match &status {
                Some(s) => {
                    let json = serde_json::json!({ "is_muted": s.is_muted });
                    InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(&message.0.correlation_id, "Audio status not yet available"),
            },
            "audio://active_sink" => match &status {
                Some(s) => match s.active_device.as_ref() {
                    Some(d) => {
                        let json = serde_json::json!({
                            "id": d.id,
                            "name": d.name.to_string(),
                            "is_default": d.is_default,
                        });
                        InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                    }
                    None => InvokeResourceResponse::success(&message.0.correlation_id, "null"),
                },
                None => InvokeResourceResponse::error(&message.0.correlation_id, "Audio status not yet available"),
            },
            "audio://sinks" => match &status {
                Some(s) => {
                    let devices: Vec<serde_json::Value> = s
                        .output_devices
                        .iter()
                        .map(|d| {
                            serde_json::json!({
                                "id": d.id,
                                "name": d.name.to_string(),
                                "is_default": d.is_default,
                            })
                        })
                        .collect();
                    let json = serde_json::Value::Array(devices);
                    InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(&message.0.correlation_id, "Audio status not yet available"),
            },
            _ => InvokeResourceResponse::error(&message.0.correlation_id, &format!("Unknown resource: {uri}")),
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
