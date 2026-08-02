use crate::config::WeatherServiceConfig;
use crate::service::WeatherService;
use crate::service::fetch_and_serialize_weather;
use crate::service::serialize_resource_state;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_weather_model::WeatherMcpResources;
use tracing::debug;

impl McpResourceHandler<WeatherMcpResources> for WeatherService {
    fn get_response(&self, request: &ResourceRequest<WeatherMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        match request.resource {
            WeatherMcpResources::NowAtCurrentLocation(coordinates) => {
                if let Some((latitude, longitude)) = coordinates {
                    let config = WeatherServiceConfig {
                        latitude,
                        longitude,
                        ..self.config.clone()
                    };
                    let result = fetch_and_serialize_weather(&config);
                    return match result {
                        Ok(contents) => InvokeResourceResponse::success(correlation_id, &contents),
                        Err(error) => InvokeResourceResponse::error(correlation_id, &error),
                    };
                }

                let state = match self.latest_state.read() {
                    Ok(state) => state.clone(),
                    Err(_) => {
                        return InvokeResourceResponse::error(correlation_id, "Failed to read weather state");
                    }
                };

                let result = serialize_resource_state(request.resource.as_ref(), &state, &self.config.location_name);
                match result {
                    Ok(contents) => InvokeResourceResponse::success(correlation_id, &contents),
                    Err(error) => InvokeResourceResponse::error(correlation_id, &error),
                }
            }
        }
    }

    fn send_resource_response(&self, response: InvokeResourceResponse, sender_id: &str) {
        self.send_response(response, sender_id);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for WeatherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        debug!("weather: InvokeResourceMessage handler uri={} sender_id={}", message.0.uri, sender_id);
        self.handle_invoke_resource_message(message, sender_id);
    }
}
