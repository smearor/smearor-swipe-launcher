use crate::service::WeatherService;
use crate::service::handle_lookup_coordinates_tool;
use crate::service::handle_lookup_location_name_tool;
use smearor_model_mcp::InvokeToolError;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_weather_model::WeatherCommandAction;
use smearor_weather_model::WeatherMcpTools;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WeatherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("weather: InvokeToolMessage handler name={} sender_id={}", tool_name, sender_id);

        let correlation_id = message.0.correlation_id.to_string();
        let broadcaster = self.get_broadcaster();
        let tool = match WeatherMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokeToolResponse::from(InvokeToolError::new(e, &correlation_id)));
                return;
            }
        };
        match tool {
            WeatherMcpTools::Refresh => {
                let _ = self.command_sender.send(WeatherCommandAction::Refresh);
                let response = InvokeToolResponse::success(&correlation_id, "Refresh triggered");
                self.send_response(response, sender_id);
            }
            WeatherMcpTools::GetForecast => {
                let arguments = message.0.arguments.to_string();
                let result = self.handle_get_forecast_tool(&arguments);
                let response = match result {
                    Ok(json) => InvokeToolResponse::success(&correlation_id, &json),
                    Err(error) => InvokeToolResponse::error(&correlation_id, &error),
                };
                self.send_response(response, sender_id);
            }
            WeatherMcpTools::GetLocation => {
                let result = self.handle_get_location_tool();
                let response = match result {
                    Ok(json) => InvokeToolResponse::success(&correlation_id, &json),
                    Err(error) => InvokeToolResponse::error(&correlation_id, &error),
                };
                self.send_response(response, sender_id);
            }
            WeatherMcpTools::LookupCoordinates => {
                let arguments = message.0.arguments.to_string();
                let result = handle_lookup_coordinates_tool(&arguments);
                let response = match result {
                    Ok(json) => InvokeToolResponse::success(&correlation_id, &json),
                    Err(error) => InvokeToolResponse::error(&correlation_id, &error),
                };
                self.send_response(response, sender_id);
            }
            WeatherMcpTools::LookupLocationName => {
                let arguments = message.0.arguments.to_string();
                let result = handle_lookup_location_name_tool(&arguments);
                let response = match result {
                    Ok(json) => InvokeToolResponse::success(&correlation_id, &json),
                    Err(error) => InvokeToolResponse::error(&correlation_id, &error),
                };
                self.send_response(response, sender_id);
            }
        }
    }
}
