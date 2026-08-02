use crate::service::MprisService;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_mpris_model::MprisMcpTools;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for MprisService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("MPRIS Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match MprisMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            MprisMcpTools::Play => {
                self.handle_play();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Playback started");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::Pause => {
                self.handle_pause();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Playback paused");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::TogglePlayPause => {
                self.handle_toggle_play_pause();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Play/pause toggled");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::Stop => {
                self.handle_stop();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Playback stopped");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::NextTrack => {
                self.handle_next_track();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Skipped to next track");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::PreviousTrack => {
                self.handle_previous_track();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Returned to previous track");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::Seek => {
                let offset = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string())
                    .ok()
                    .and_then(|v| v.get("offset").and_then(|a| a.as_i64()))
                    .unwrap_or(0);
                self.handle_seek(offset);
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Seeked by {offset} microseconds"));
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::SetPosition => {
                let position = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string())
                    .ok()
                    .and_then(|v| v.get("position").and_then(|a| a.as_i64()))
                    .unwrap_or(0);
                self.handle_set_position(position);
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Position set to {position} microseconds"));
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::CycleLoop => {
                self.handle_cycle_loop();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Loop mode cycled");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::ToggleShuffle => {
                self.handle_toggle_shuffle();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Shuffle toggled");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::NextPlayer => {
                self.handle_next_player();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Switched to next player");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::PreviousPlayer => {
                self.handle_previous_player();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Switched to previous player");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::Raise => {
                self.handle_raise();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Player window raised");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::Quit => {
                self.handle_quit();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Player application quit");
                broadcaster.broadcast_message_to_topic(response);
            }
            MprisMcpTools::RefreshStatus => {
                let _ = self.command_sender.send(crate::mpris_command::MprisCommand::RefreshStatus);
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Status refresh triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
