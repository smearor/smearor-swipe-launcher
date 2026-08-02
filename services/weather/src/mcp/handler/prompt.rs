use crate::service::WeatherService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_weather_model::WeatherMcpPrompts;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for WeatherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("weather: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match WeatherMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let response = match prompt {
            WeatherMcpPrompts::WeatherQueryGuide => {
                let arguments = message.0.arguments.to_string();
                let include_forecast = serde_json::from_str::<serde_json::Value>(&arguments)
                    .ok()
                    .and_then(|v| v.get("include_forecast").and_then(|a| a.as_bool()))
                    .unwrap_or(true);

                let mut content = format!(
                    "You can query weather using the 'weather_get_forecast' tool \
                     (configured location: {}, {}). Pass latitude and longitude for custom coordinates.",
                    self.config.latitude, self.config.longitude
                );

                if include_forecast {
                    content.push_str(" The tool provides current conditions and forecast data.");
                }

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
            WeatherMcpPrompts::WeatherContextGuide => {
                let content = format!(
                    "Weather location resolution guide:\n\
                     \n\
                     1. If the user provides coordinates (lat/lon), use them directly with weather_get_forecast.\n\
                     2. If the user provides a place name, use weather_lookup_coordinates to resolve it to lat/lon first.\n\
                     3. If no location is given, use the configured default location: lat={}, lon={}.\n\
                     4. To get current weather or forecast for any location, use the tool 'weather_get_forecast' with latitude and longitude.\n\
                     5. To reverse-lookup a location name from coordinates, use weather_lookup_location_name.\n\
                     \n\
                     Always confirm the location with the user before making API calls if ambiguous.",
                    self.config.latitude, self.config.longitude
                );

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };
        self.send_response(response, sender_id);
    }
}
