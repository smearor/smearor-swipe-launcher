use crate::config::WeatherServiceConfig;
use crate::fetcher::FetchResult;
use crate::fetcher::WeatherFetcher;
use crate::fetcher::map_current_weather;
use crate::fetcher::map_daily_forecast_data;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_weather_model::WeatherCommandAction;
use smearor_weather_model::WeatherCommandMessage;
use smearor_weather_model::WeatherStatusMessage;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::trace;

/// Latest weather state shared between the update loop and MCP handlers.
#[derive(Clone, Default)]
pub struct LatestWeatherState {
    /// Last successful weather status message.
    pub status: WeatherStatusMessage,
}

pub struct WeatherService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WeatherServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<WeatherCommandAction>,
    pub latest_state: Arc<RwLock<LatestWeatherState>>,
}

impl WeatherService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let service_config: WeatherServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<WeatherCommandAction>();
        let meta = PluginMeta::try_from(&config)?;
        let meta_clone = meta.clone();
        let core_context_clone = core_context;
        let service_config_clone = service_config.clone();
        let latest_state = Arc::new(RwLock::new(LatestWeatherState::default()));
        let latest_state_clone = latest_state.clone();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    debug!("Weather service: failed to create tokio runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_update_loop(service_config_clone, command_receiver, meta_clone, core_context_clone, latest_state_clone).await;
            });
        });

        let service = WeatherService {
            meta,
            core_context,
            config: service_config,
            command_sender,
            latest_state,
        };
        service.register_mcp_capabilities();
        Ok(service)
    }

    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resource = RegisterResourceMessage::new(
            "weather://current",
            "Current Weather",
            "Current weather conditions for the configured location.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(resource);

        let tool = RegisterToolMessage::new("weather_refresh", "Force an immediate refresh of weather data.", r#"{ "type": "object", "properties": {} }"#);
        broadcaster.broadcast_message_to_topic(tool);

        let forecast_tool = RegisterToolMessage::new(
            "weather_get_forecast",
            "Get weather forecast for arbitrary coordinates.",
            r#"{ "type": "object", "properties": { "latitude": { "type": "number" }, "longitude": { "type": "number" } }, "required": ["latitude", "longitude"] }"#,
        );
        broadcaster.broadcast_message_to_topic(forecast_tool);
    }

    fn send_response<T: TypedMessage + SharedMessage + Clone>(&self, message: T, sender_id: &str) {
        let payload_ptr = Box::into_raw(Box::new(message.clone())) as *mut core::ffi::c_void;
        let sender_id_string = sender_id.to_string();
        let topic = message.topic();
        debug!("weather: send_response topic={} to sender_id={}", topic, sender_id);
        let envelope = FfiEnvelope {
            sender_id: self.meta.id.clone(),
            target_instance_id: stabby::string::String::from(sender_id_string.as_str()),
            topic: stabby::string::String::from(topic),
            type_id: T::TYPE_ID,
            payload: payload_ptr,
            destroy_payload: Some(destroy_payload::<T>),
            clone_payload: Some(clone_payload::<T>),
        };
        if let Some(context) = &self.core_context {
            context.send_message(envelope);
        } else {
            debug!("weather: no core_context, cannot send response");
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<WeatherCommandMessage>> for WeatherService {
    fn handle_message(&self, message: FfiEnvelopePayload<WeatherCommandMessage>, _sender_id: &str) {
        match message.action {
            WeatherCommandAction::Refresh => {
                let _ = self.command_sender.send(WeatherCommandAction::Refresh);
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for WeatherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("weather: InvokeToolMessage handler name={} sender_id={}", tool_name, sender_id);

        if tool_name == "weather_refresh" {
            let _ = self.command_sender.send(WeatherCommandAction::Refresh);
            let correlation_id = message.0.correlation_id.to_string();
            let response = InvokeToolResponse::success(&correlation_id, "Refresh triggered");
            self.send_response(response, sender_id);
        } else if tool_name == "weather_get_forecast" {
            let correlation_id = message.0.correlation_id.to_string();
            let arguments = message.0.arguments.to_string();
            let result = handle_get_forecast_tool(&arguments);
            let response = match result {
                Ok(json) => InvokeToolResponse::success(&correlation_id, &json),
                Err(error) => InvokeToolResponse::error(&correlation_id, &error),
            };
            self.send_response(response, sender_id);
        } else {
            debug!("weather: unknown tool name '{}', ignoring", tool_name);
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for WeatherService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        let uri = message.0.uri.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("weather: InvokeResourceMessage handler uri={} sender_id={}", uri, sender_id);

        let state = match self.latest_state.read() {
            Ok(state) => state.clone(),
            Err(_) => {
                let response = InvokeResourceResponse::error(&correlation_id, "Failed to read weather state");
                self.send_response(response, sender_id);
                return;
            }
        };

        let result = serialize_resource_state(&uri, &state);
        let response = match result {
            Ok(contents) => InvokeResourceResponse::success(&correlation_id, &contents),
            Err(error) => InvokeResourceResponse::error(&correlation_id, &error),
        };
        self.send_response(response, sender_id);
    }
}

impl MessageBroadcaster for WeatherService {}

impl MessageTopicBroadcaster<WeatherStatusMessage> for WeatherService {}

impl PluginMetaGetter for WeatherService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WeatherService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for WeatherService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                debug!("weather: on_message topic={} type_id={}", envelope.topic, envelope.type_id);
                if envelope.type_id == FfiEnvelopePayload::<WeatherCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<WeatherCommandMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else {
                    debug!("weather: unknown type_id");
                }
            }
        }
    }
}

async fn run_update_loop(
    config: WeatherServiceConfig,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<WeatherCommandAction>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    latest_state: Arc<RwLock<LatestWeatherState>>,
) {
    let fetcher = WeatherFetcher::new();
    let interval_minutes = config.update_interval_minutes.max(10);
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_minutes * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            command = command_receiver.recv() => {
                if command.is_none() {
                    break;
                }
            }
        }

        let fetch_result = fetcher.fetch(&config).await;
        let now = chrono::Utc::now().to_rfc3339();

        let status_message = match fetch_result {
            Ok(FetchResult { forecast, air_quality }) => {
                let current = map_current_weather(&forecast);
                let daily = map_daily_forecast_data(&forecast);
                let air_quality_stabby = air_quality.map(stabby::option::Option::Some).unwrap_or(stabby::option::Option::None());
                WeatherStatusMessage {
                    latitude: forecast.latitude,
                    longitude: forecast.longitude,
                    current,
                    daily,
                    air_quality: air_quality_stabby,
                    last_updated: stabby::string::String::from(now.as_str()),
                    success: true,
                    is_stale: false,
                    error_message: stabby::option::Option::None(),
                }
            }
            Err(error) => {
                debug!("Weather service: fetch error: {error}");
                let previous_status = {
                    let state = latest_state.read();
                    state.map(|s| s.status.clone()).unwrap_or_default()
                };
                if previous_status.success {
                    WeatherStatusMessage {
                        is_stale: true,
                        error_message: stabby::option::Option::Some(stabby::string::String::from(error.as_str())),
                        last_updated: stabby::string::String::from(now.as_str()),
                        ..previous_status
                    }
                } else {
                    WeatherStatusMessage {
                        success: false,
                        is_stale: false,
                        error_message: stabby::option::Option::Some(stabby::string::String::from(error.as_str())),
                        last_updated: stabby::string::String::from(now.as_str()),
                        ..Default::default()
                    }
                }
            }
        };

        {
            if let Ok(mut guard) = latest_state.write() {
                guard.status = status_message.clone();
            }
        }

        broadcast(&meta, &core_context, status_message);
        trace!("Weather service broadcasted status");
    }
}

fn broadcast<T: Clone + MessageTopic + TypedMessage>(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, message: T) {
    let payload_ptr = Box::into_raw(Box::new(message.clone())) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: meta.id.clone(),
        target_instance_id: stabby::string::String::from(""),
        topic: stabby::string::String::from(T::topic()),
        type_id: T::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(destroy_payload::<T>),
        clone_payload: Some(clone_payload::<T>),
    };

    if let Some(context) = core_context {
        context.send_message(envelope);
    }
}

extern "C" fn clone_payload<T: Clone>(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let value = unsafe { &*(ptr as *const T) };
    Box::into_raw(Box::new(value.clone())) as *mut core::ffi::c_void
}

extern "C" fn destroy_payload<T>(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut T);
        }
    }
}

fn serialize_resource_state(uri: &str, state: &LatestWeatherState) -> Result<String, String> {
    let status = &state.status;
    match uri {
        "weather://current" => {
            let current = &status.current;
            let air_quality = &status.air_quality;
            let json = serde_json::json!({
                "latitude": status.latitude,
                "longitude": status.longitude,
                "last_updated": status.last_updated.to_string(),
                "success": status.success,
                "is_stale": status.is_stale,
                "error": status.error_message.as_ref().map(|e| e.to_string()),
                "current": {
                    "temperature": current.temperature.as_ref().copied(),
                    "cloud_cover": current.cloud_cover.as_ref().copied(),
                    "relative_humidity": current.relative_humidity.as_ref().copied(),
                    "wind_speed": current.wind_speed.as_ref().copied(),
                    "wind_direction": current.wind_direction.as_ref().copied(),
                    "pressure": current.pressure.as_ref().copied(),
                    "uv_index": current.uv_index.as_ref().copied(),
                    "weather_code": current.weather_code.as_ref().copied(),
                    "is_day": current.is_day.as_ref().copied(),
                },
                "daily": {
                    "today": {
                        "temperature_max": status.daily.today.temperature_max.as_ref().copied(),
                        "temperature_min": status.daily.today.temperature_min.as_ref().copied(),
                        "weather_code": status.daily.today.weather_code.as_ref().copied(),
                    },
                    "tomorrow": {
                        "temperature_max": status.daily.tomorrow.temperature_max.as_ref().copied(),
                        "temperature_min": status.daily.tomorrow.temperature_min.as_ref().copied(),
                        "weather_code": status.daily.tomorrow.weather_code.as_ref().copied(),
                    },
                },
                "air_quality": air_quality.as_ref().map(|aq| serde_json::json!({
                    "european_aqi": aq.european_aqi.as_ref().copied(),
                    "pm10": aq.pm10.as_ref().copied(),
                    "pm2_5": aq.pm2_5.as_ref().copied(),
                    "ozone": aq.ozone.as_ref().copied(),
                    "nitrogen_dioxide": aq.nitrogen_dioxide.as_ref().copied(),
                    "sulphur_dioxide": aq.sulphur_dioxide.as_ref().copied(),
                    "carbon_monoxide": aq.carbon_monoxide.as_ref().copied(),
                })),
            });
            Ok(json.to_string())
        }
        _ => Err(format!("Unknown resource uri: {uri}")),
    }
}

fn handle_get_forecast_tool(arguments: &str) -> Result<String, String> {
    let args: serde_json::Value = serde_json::from_str(arguments).map_err(|e| format!("Invalid arguments: {e}"))?;
    let latitude = args.get("latitude").and_then(|v| v.as_f64()).ok_or("Missing 'latitude' parameter")?;
    let longitude = args.get("longitude").and_then(|v| v.as_f64()).ok_or("Missing 'longitude' parameter")?;

    let config = WeatherServiceConfig {
        latitude,
        longitude,
        ..Default::default()
    };

    let fetcher = WeatherFetcher::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {e}"))?;

    let result = runtime.block_on(async move { fetcher.fetch(&config).await })?;

    let current = map_current_weather(&result.forecast);
    let daily = map_daily_forecast_data(&result.forecast);

    let json = serde_json::json!({
        "latitude": result.forecast.latitude,
        "longitude": result.forecast.longitude,
        "current": {
            "temperature": current.temperature.as_ref().copied(),
            "cloud_cover": current.cloud_cover.as_ref().copied(),
            "relative_humidity": current.relative_humidity.as_ref().copied(),
            "wind_speed": current.wind_speed.as_ref().copied(),
            "wind_direction": current.wind_direction.as_ref().copied(),
            "pressure": current.pressure.as_ref().copied(),
            "uv_index": current.uv_index.as_ref().copied(),
            "weather_code": current.weather_code.as_ref().copied(),
            "is_day": current.is_day.as_ref().copied(),
        },
        "daily": {
            "today": {
                "temperature_max": daily.today.temperature_max.as_ref().copied(),
                "temperature_min": daily.today.temperature_min.as_ref().copied(),
                "weather_code": daily.today.weather_code.as_ref().copied(),
            },
            "tomorrow": {
                "temperature_max": daily.tomorrow.temperature_max.as_ref().copied(),
                "temperature_min": daily.tomorrow.temperature_min.as_ref().copied(),
                "weather_code": daily.tomorrow.weather_code.as_ref().copied(),
            },
        },
        "air_quality": result.air_quality.as_ref().map(|aq| serde_json::json!({
            "european_aqi": aq.european_aqi.as_ref().copied(),
            "pm10": aq.pm10.as_ref().copied(),
            "pm2_5": aq.pm2_5.as_ref().copied(),
        })),
    });
    Ok(json.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_resource_unknown_uri_returns_error() {
        let state = LatestWeatherState::default();
        let result = serialize_resource_state("weather://unknown", &state);
        assert!(result.is_err());
    }

    #[test]
    fn serialize_resource_current_returns_json() {
        let mut state = LatestWeatherState::default();
        state.status.latitude = 52.52;
        state.status.longitude = 13.41;
        state.status.success = true;
        let result = serialize_resource_state("weather://current", &state);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("\"latitude\":52.52"));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn handle_get_forecast_missing_latitude_returns_error() {
        let result = handle_get_forecast_tool(r#"{"longitude": 13.41}"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("latitude"));
    }
}
