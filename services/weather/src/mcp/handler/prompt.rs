use crate::service::WeatherService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_model_mcp::render_template;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
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

                let mut content = render_template(
                    include_str!("../../../data/prompts/weather_query_guide.md"),
                    &[
                        ("latitude", &self.config.latitude.to_string()),
                        ("longitude", &self.config.longitude.to_string()),
                    ],
                );

                if include_forecast {
                    content.push_str(" The tool provides current conditions and forecast data.");
                }

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
            WeatherMcpPrompts::WeatherContextGuide => {
                let content = render_template(
                    include_str!("../../../data/prompts/weather_context_guide.md"),
                    &[
                        ("latitude", &self.config.latitude.to_string()),
                        ("longitude", &self.config.longitude.to_string()),
                    ],
                );

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };
        self.send_response(response, sender_id);
    }
}
