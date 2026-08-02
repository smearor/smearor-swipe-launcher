use crate::command::PersonalizationCommand;
use crate::service::PersonalizationService;
use crate::service::parse_coordinates;
use crate::service::parse_locale;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_personalization_model::PersonalizationMcpTools;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for PersonalizationService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        trace!("personalization: InvokeToolMessage handler name={} sender_id={}", tool_name, sender_id);

        let broadcaster = self.get_broadcaster();
        let tool = match PersonalizationMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &correlation_id)));
                return;
            }
        };
        match tool {
            PersonalizationMcpTools::GetCurrentLocation => {
                let state = self.latest_state.read().map(|s| s.clone()).unwrap_or_default();
                let coords = &state.status.coordinates;
                let json = serde_json::json!({
                    "latitude": coords.as_ref().map(|c| c.latitude).unwrap_or(0.0),
                    "longitude": coords.as_ref().map(|c| c.longitude).unwrap_or(0.0),
                    "location_name": coords.as_ref().and_then(|c| c.location_name.as_ref().map(|n| n.to_string())),
                });
                let response = InvokeToolResponse::success(&correlation_id, &json.to_string());
                self.send_response(response, sender_id);
            }
            PersonalizationMcpTools::GetTimezone => {
                let state = self.latest_state.read().map(|s| s.clone()).unwrap_or_default();
                let timezone = state.status.timezone.as_ref().map(|t| t.to_string()).unwrap_or_default();
                let json = serde_json::json!({ "timezone": timezone });
                let response = InvokeToolResponse::success(&correlation_id, &json.to_string());
                self.send_response(response, sender_id);
            }
            PersonalizationMcpTools::GetLocale => {
                let state = self.latest_state.read().map(|s| s.clone()).unwrap_or_default();
                let locale = state.status.locale.as_ref().map(|l| l.to_string()).unwrap_or_default();
                let json = serde_json::json!({ "locale": locale });
                let response = InvokeToolResponse::success(&correlation_id, &json.to_string());
                self.send_response(response, sender_id);
            }
            PersonalizationMcpTools::GetPersonalization => {
                let state = self.latest_state.read().map(|s| s.clone()).unwrap_or_default();
                let json = serde_json::to_string(&state.status).unwrap_or_else(|_| "{}".to_string());
                let response = InvokeToolResponse::success(&correlation_id, &json);
                self.send_response(response, sender_id);
            }
            PersonalizationMcpTools::SetCurrentLocation => {
                let arguments = message.0.arguments.to_string();
                let result = parse_coordinates(&arguments);
                let response = match result {
                    Ok(coords) => {
                        let _ = self.command_sender.send(PersonalizationCommand::UpdateLocation(coords.clone()));
                        let json = serde_json::json!({
                            "latitude": coords.latitude,
                            "longitude": coords.longitude,
                            "location_name": coords.location_name.as_ref().map(|n| n.to_string()),
                        });
                        InvokeToolResponse::success(&correlation_id, &json.to_string())
                    }
                    Err(error) => InvokeToolResponse::error(&correlation_id, &error),
                };
                self.send_response(response, sender_id);
            }
            PersonalizationMcpTools::SetLocale => {
                let arguments = message.0.arguments.to_string();
                let result = parse_locale(&arguments);
                let response = match result {
                    Ok(locale) => {
                        let _ = self.command_sender.send(PersonalizationCommand::UpdateLocale(locale.clone()));
                        let json = serde_json::json!({ "locale": locale });
                        InvokeToolResponse::success(&correlation_id, &json.to_string())
                    }
                    Err(error) => InvokeToolResponse::error(&correlation_id, &error),
                };
                self.send_response(response, sender_id);
            }
            PersonalizationMcpTools::RefreshPersonalization => {
                let _ = self.command_sender.send(PersonalizationCommand::Refresh);
                let response = InvokeToolResponse::success(&correlation_id, "Refresh triggered. Runtime overrides cleared.");
                self.send_response(response, sender_id);
            }
        }
    }
}
