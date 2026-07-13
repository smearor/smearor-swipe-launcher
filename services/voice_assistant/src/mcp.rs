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

use crate::service::VoiceAssistantService;

impl VoiceAssistantService {
    /// Registers MCP resources and tools for the voice assistant service.
    pub fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let status_resource = RegisterResourceMessage::new(
            "voice_assistant://status",
            "Voice Assistant Status",
            "Current assistant state, transcript, and final answer.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(status_resource);

        let tool_catalog_resource = RegisterResourceMessage::new(
            "voice_assistant://tool_catalog",
            "Voice Assistant Tool Catalog",
            "List of all discovered tools in the catalog.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(tool_catalog_resource);

        let activate_tool = RegisterToolMessage::new(
            "voice_assistant_activate",
            "Starts audio capture and begins the voice pipeline.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(activate_tool);

        let deactivate_tool = RegisterToolMessage::new(
            "voice_assistant_deactivate",
            "Stops audio capture and cancels the pipeline.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(deactivate_tool);

        let submit_text_tool = RegisterToolMessage::new(
            "voice_assistant_submit_text",
            "Submits a text command directly (bypassing STT).",
            r#"{ "type": "object", "properties": { "text": { "type": "string", "description": "The text command to submit" } }, "required": ["text"] }"#,
        );
        broadcaster.broadcast_message_to_topic(submit_text_tool);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Voice Assistant Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();

        match tool_name.as_str() {
            "voice_assistant_activate" => {
                self.activate();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Voice assistant activated");
                broadcaster.broadcast_message_to_topic(response);
            }
            "voice_assistant_deactivate" => {
                self.deactivate();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Voice assistant deactivated");
                broadcaster.broadcast_message_to_topic(response);
            }
            "voice_assistant_submit_text" => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let text = args.get("text").and_then(|v| v.as_str());
                match text {
                    Some(text) => {
                        self.submit_text(text);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Text submitted");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: text");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            _ => {
                let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Unknown tool: {tool_name}"));
                broadcaster.broadcast_message_to_topic(response);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, _sender_id: &str) {
        let uri = message.0.uri.to_string();
        debug!("Voice Assistant Service: InvokeResourceMessage uri={}", uri);
        let broadcaster = self.get_broadcaster();

        let response = match uri.as_str() {
            "voice_assistant://status" => {
                let state = self.state.read().map(|state| format!("{:?}", *state)).unwrap_or_else(|_| "Unknown".to_string());
                let json = serde_json::json!({
                    "state": state,
                    "transcript": self.current_transcript.read().map(|t| t.clone()).unwrap_or_default(),
                    "final_answer": self.current_answer.read().map(|a| a.clone()).unwrap_or_default(),
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            "voice_assistant://tool_catalog" => {
                let catalog = self.tool_catalog.read().unwrap_or_else(|e| e.into_inner());
                let json = serde_json::json!({
                    "tools": catalog.iter().map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "input_schema": t.input_schema,
                        })
                    }).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(&message.0.correlation_id, &json.to_string())
            }
            _ => InvokeResourceResponse::error(&message.0.correlation_id, &format!("Unknown resource: {uri}")),
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
