use crate::service::AudioService;
use smearor_audio_model::AudioMcpTools;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for AudioService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Audio Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match AudioMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &message.0.correlation_id)));
                return;
            }
        };
        match tool {
            AudioMcpTools::VolumeUp => {
                self.handle_volume_up();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Volume increased");
                broadcaster.broadcast_message_to_topic(response);
            }
            AudioMcpTools::VolumeDown => {
                self.handle_volume_down();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Volume decreased");
                broadcaster.broadcast_message_to_topic(response);
            }
            AudioMcpTools::SetVolume => {
                let volume = serde_json::from_str::<serde_json::Value>(&message.0.arguments.to_string())
                    .ok()
                    .and_then(|v| v.get("volume").and_then(|a| a.as_f64()).map(|f| f as f32))
                    .unwrap_or(0.0);
                self.handle_set_volume(volume);
                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Volume set to {volume}"));
                broadcaster.broadcast_message_to_topic(response);
            }
            AudioMcpTools::ToggleMute => {
                self.handle_toggle_mute();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Mute toggled");
                broadcaster.broadcast_message_to_topic(response);
            }
            AudioMcpTools::Mute => {
                self.handle_mute();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Audio muted");
                broadcaster.broadcast_message_to_topic(response);
            }
            AudioMcpTools::Unmute => {
                self.handle_unmute();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Audio unmuted");
                broadcaster.broadcast_message_to_topic(response);
            }
            AudioMcpTools::NextDevice => {
                self.handle_next_device();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Switched to next device");
                broadcaster.broadcast_message_to_topic(response);
            }
            AudioMcpTools::PreviousDevice => {
                self.handle_previous_device();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Switched to previous device");
                broadcaster.broadcast_message_to_topic(response);
            }
            AudioMcpTools::RefreshStatus => {
                self.handle_refresh_status();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Status refresh triggered");
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}
