use crate::config::WeatherServiceConfig;
use crate::fetcher::FetchResult;
use crate::fetcher::WeatherFetcher;
use crate::fetcher::map_current_weather;
use crate::fetcher::map_daily_forecast_data;
use crate::latest_state::LatestWeatherState;
use crate::personalization_coordinates::PersonalizationCoordinates;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use smearor_weather_model::TOPIC_COMMAND;
use smearor_weather_model::VoiceDescribable;
use smearor_weather_model::WeatherCommandAction;
use smearor_weather_model::WeatherCommandMessage;
use smearor_weather_model::WeatherStatusMessage;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::trace;

const OPEN_METEO_GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const NOMINATIM_REVERSE_URL: &str = "https://nominatim.openstreetmap.org/reverse";

pub struct WeatherService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    #[allow(unused)]
    pub config: WeatherServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<WeatherCommandAction>,
    pub latest_state: Arc<RwLock<LatestWeatherState>>,
    personalization_coords: Arc<RwLock<PersonalizationCoordinates>>,
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
        let personalization_coords = Arc::new(RwLock::new(PersonalizationCoordinates::default()));
        let personalization_coords_clone = personalization_coords.clone();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    debug!("Weather service: failed to create tokio runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_update_loop(
                    service_config_clone,
                    command_receiver,
                    meta_clone,
                    core_context_clone,
                    latest_state_clone,
                    personalization_coords_clone,
                )
                .await;
            });
        });

        let service = WeatherService {
            meta,
            core_context,
            config: service_config,
            command_sender,
            latest_state,
            personalization_coords,
        };
        service.register_mcp_capabilities();
        Ok(service)
    }

    pub(crate) fn send_response<T: TypedMessage + SharedMessage + Clone>(&self, message: T, sender_id: &str) {
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
            destroy_payload: Some(default_destroy_payload),
            clone_payload: Some(default_clone_payload::<T>),
        };
        if let Some(context) = &self.core_context {
            context.send_message(envelope);
        } else {
            debug!("weather: no core_context, cannot send response");
        }
    }

    pub(crate) fn handle_get_forecast_tool(&self, arguments: &str) -> Result<String, String> {
        handle_get_forecast_tool(arguments, &self.config)
    }

    pub(crate) fn handle_get_location_tool(&self) -> Result<String, String> {
        let json = serde_json::json!({
            "location_name": self.config.location_name,
            "latitude": self.config.latitude,
            "longitude": self.config.longitude,
            "timezone": self.config.timezone,
        });
        Ok(json.to_string())
    }
}

impl AcceptTopic<FfiEnvelope> for WeatherService {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_PERSONALIZATION_STATUS
            || topic == TOPIC_COMMAND
            || topic == TOPIC_MCP_INVOKE_TOOL
            || topic == TOPIC_MCP_INVOKE_RESOURCE
            || topic == TOPIC_MCP_INVOKE_PROMPT
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for WeatherService {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("weather: received personalization status");
        let status = message.0;
        let new_coords = PersonalizationCoordinates {
            latitude: status.coordinates.as_ref().map(|c| c.latitude),
            longitude: status.coordinates.as_ref().map(|c| c.longitude),
        };

        let coords_changed = {
            if let Ok(old) = self.personalization_coords.read() {
                old.latitude != new_coords.latitude || old.longitude != new_coords.longitude
            } else {
                true
            }
        };

        if let Ok(mut guard) = self.personalization_coords.write() {
            *guard = new_coords;
        }

        if coords_changed {
            debug!("weather: personalization coordinates changed, triggering refresh");
            let _ = self.command_sender.send(WeatherCommandAction::Refresh);
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

impl ServicePlugin for WeatherService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                trace!("weather: on_message topic={} type_id={}", topic, envelope.type_id);
                if envelope.type_id == FfiEnvelopePayload::<PersonalizationStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<WeatherCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<WeatherCommandMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_PROMPT && envelope.type_id == FfiEnvelopePayload::<InvokePromptMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokePromptMessage>>::handle_envelope_message(self, envelope);
                } else {
                    trace!("weather: unknown type_id");
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
    personalization_coords: Arc<RwLock<PersonalizationCoordinates>>,
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

        let effective_config = if config.use_personalization {
            if let Ok(coords) = personalization_coords.read() {
                if let (Some(lat), Some(lon)) = (coords.latitude, coords.longitude) {
                    WeatherServiceConfig {
                        latitude: lat,
                        longitude: lon,
                        ..config.clone()
                    }
                } else {
                    config.clone()
                }
            } else {
                config.clone()
            }
        } else {
            config.clone()
        };

        let fetch_result = fetcher.fetch(&effective_config).await;
        let now = chrono::Utc::now().to_rfc3339();

        let status_message = match fetch_result {
            Ok(FetchResult { forecast, air_quality }) => {
                let current = map_current_weather(&forecast);
                let daily = map_daily_forecast_data(&forecast);
                let air_quality_stabby = air_quality.map(stabby::option::Option::Some).unwrap_or(stabby::option::Option::None());
                WeatherStatusMessage {
                    latitude: forecast.latitude,
                    longitude: forecast.longitude,
                    elevation: forecast
                        .elevation
                        .map(|e| stabby::option::Option::Some(e as f32))
                        .unwrap_or(stabby::option::Option::None()),
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
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<T>),
    };

    if let Some(context) = core_context {
        context.send_message(envelope);
    }
}

pub(crate) fn serialize_resource_state(uri: &str, state: &LatestWeatherState, location_name: &Option<String>) -> Result<String, String> {
    let status = &state.status;
    match uri {
        "weather://now_at_current_location" => {
            let current = &status.current;
            let air_quality = &status.air_quality;
            let json = serde_json::json!({
                "location_name": location_name,
                "latitude": status.latitude,
                "longitude": status.longitude,
                "elevation": status.elevation.as_ref().copied(),
                "last_updated": status.last_updated.to_string(),
                "success": status.success,
                "is_stale": status.is_stale,
                "error": status.error_message.as_ref().map(|e| e.to_string()),
                "current": {
                    "temperature": current.temperature.as_ref().copied(),
                    "temperature_text": current.temperature_level().map(|l| l.to_string()),
                    "cloud_cover": current.cloud_cover.as_ref().copied(),
                    "cloud_cover_text": current.cloud_cover_level().map(|l| l.to_string()),
                    "relative_humidity": current.relative_humidity.as_ref().copied(),
                    "relative_humidity_text": current.humidity_level().map(|l| l.to_string()),
                    "wind_speed": current.wind_speed.as_ref().copied(),
                    "wind_speed_text": current.wind_speed_level().map(|l| l.to_string()),
                    "wind_direction": current.wind_direction.as_ref().copied(),
                    "wind_direction_text": current.wind_direction().map(|d| d.to_string()),
                    "pressure": current.pressure.as_ref().copied(),
                    "pressure_text": current.pressure_level().map(|l| l.to_string()),
                    "uv_index": current.uv_index.as_ref().copied(),
                    "uv_index_text": current.uv_index_level().map(|l| l.to_string()),
                    "weather_code": current.weather_code.as_ref().copied(),
                    "is_day": current.is_day.as_ref().copied(),
                    "showers": current.showers.as_ref().copied(),
                    "snowfall": current.snowfall.as_ref().copied(),
                    "rain": current.rain.as_ref().copied(),
                    "precipitation": current.precipitation.as_ref().copied(),
                    "precipitation_text": current.precipitation_intensity().map(|l| l.to_string()),
                },
                "daily": {
                    "today": {
                        "temperature_max": status.daily.today.temperature_max.as_ref().copied(),
                        "temperature_min": status.daily.today.temperature_min.as_ref().copied(),
                        "weather_code": status.daily.today.weather_code.as_ref().copied(),
                        "daylight_duration": status.daily.today.daylight_duration.as_ref().copied(),
                        "sunshine_duration": status.daily.today.sunshine_duration.as_ref().copied(),
                        "sunshine_text": status.daily.today.sunshine_level().map(|l| l.to_string()),
                        "precipitation_sum": status.daily.today.precipitation_sum.as_ref().copied(),
                        "precipitation_sum_text": status.daily.today.precipitation_amount_level().map(|l| l.to_string()),
                        "precipitation_hours": status.daily.today.precipitation_hours.as_ref().copied(),
                        "precipitation_probability_max": status.daily.today.precipitation_probability_max.as_ref().copied(),
                        "precipitation_probability_text": status.daily.today.precipitation_probability_level().map(|l| l.to_string()),
                    },
                    "tomorrow": {
                        "temperature_max": status.daily.tomorrow.temperature_max.as_ref().copied(),
                        "temperature_min": status.daily.tomorrow.temperature_min.as_ref().copied(),
                        "weather_code": status.daily.tomorrow.weather_code.as_ref().copied(),
                        "daylight_duration": status.daily.tomorrow.daylight_duration.as_ref().copied(),
                        "sunshine_duration": status.daily.tomorrow.sunshine_duration.as_ref().copied(),
                        "sunshine_text": status.daily.tomorrow.sunshine_level().map(|l| l.to_string()),
                        "precipitation_sum": status.daily.tomorrow.precipitation_sum.as_ref().copied(),
                        "precipitation_sum_text": status.daily.tomorrow.precipitation_amount_level().map(|l| l.to_string()),
                        "precipitation_hours": status.daily.tomorrow.precipitation_hours.as_ref().copied(),
                        "precipitation_probability_max": status.daily.tomorrow.precipitation_probability_max.as_ref().copied(),
                        "precipitation_probability_text": status.daily.tomorrow.precipitation_probability_level().map(|l| l.to_string()),
                    },
                },
                "air_quality": air_quality.as_ref().map(|aq| serde_json::json!({
                    "european_aqi": aq.european_aqi.as_ref().copied(),
                    "european_aqi_text": aq.air_quality_level().map(|l| l.to_string()),
                    "pm10": aq.pm10.as_ref().copied(),
                    "pm2_5": aq.pm2_5.as_ref().copied(),
                    "pm2_5_text": aq.particulate_matter_level().map(|l| l.to_string()),
                    "ozone": aq.ozone.as_ref().copied(),
                    "nitrogen_dioxide": aq.nitrogen_dioxide.as_ref().copied(),
                    "sulphur_dioxide": aq.sulphur_dioxide.as_ref().copied(),
                    "carbon_monoxide": aq.carbon_monoxide.as_ref().copied(),
                })),
                "voice": {
                    "current_summary": current.voice_summary(),
                    "today_summary": status.daily.today.voice_summary(),
                    "tomorrow_summary": status.daily.tomorrow.voice_summary(),
                    "air_quality_summary": air_quality.as_ref().map(|aq| aq.voice_summary()),
                },
            });
            Ok(json.to_string())
        }
        _ => Err(format!("Unknown resource uri: {uri}")),
    }
}

pub(crate) fn fetch_and_serialize_weather(config: &WeatherServiceConfig) -> Result<String, String> {
    let fetcher = WeatherFetcher::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {e}"))?;

    let result = runtime.block_on(async move { fetcher.fetch(config).await })?;
    serialize_fetch_result(&result, config.latitude, config.longitude)
}

fn serialize_fetch_result(result: &FetchResult, latitude: f64, longitude: f64) -> Result<String, String> {
    let current = map_current_weather(&result.forecast);
    let daily = map_daily_forecast_data(&result.forecast);

    let json = serde_json::json!({
        "latitude": latitude,
        "longitude": longitude,
        "elevation": result.forecast.elevation,
        "current": {
            "temperature": current.temperature.as_ref().copied(),
            "temperature_text": current.temperature_level().map(|l| l.to_string()),
            "cloud_cover": current.cloud_cover.as_ref().copied(),
            "cloud_cover_text": current.cloud_cover_level().map(|l| l.to_string()),
            "relative_humidity": current.relative_humidity.as_ref().copied(),
            "relative_humidity_text": current.humidity_level().map(|l| l.to_string()),
            "wind_speed": current.wind_speed.as_ref().copied(),
            "wind_speed_text": current.wind_speed_level().map(|l| l.to_string()),
            "wind_direction": current.wind_direction.as_ref().copied(),
            "wind_direction_text": current.wind_direction().map(|d| d.to_string()),
            "pressure": current.pressure.as_ref().copied(),
            "pressure_text": current.pressure_level().map(|l| l.to_string()),
            "uv_index": current.uv_index.as_ref().copied(),
            "uv_index_text": current.uv_index_level().map(|l| l.to_string()),
            "weather_code": current.weather_code.as_ref().copied(),
            "is_day": current.is_day.as_ref().copied(),
            "showers": current.showers.as_ref().copied(),
            "snowfall": current.snowfall.as_ref().copied(),
            "rain": current.rain.as_ref().copied(),
            "precipitation": current.precipitation.as_ref().copied(),
            "precipitation_text": current.precipitation_intensity().map(|l| l.to_string()),
        },
        "daily": {
            "today": {
                "temperature_max": daily.today.temperature_max.as_ref().copied(),
                "temperature_min": daily.today.temperature_min.as_ref().copied(),
                "weather_code": daily.today.weather_code.as_ref().copied(),
                "daylight_duration": daily.today.daylight_duration.as_ref().copied(),
                "sunshine_duration": daily.today.sunshine_duration.as_ref().copied(),
                "sunshine_text": daily.today.sunshine_level().map(|l| l.to_string()),
                "precipitation_sum": daily.today.precipitation_sum.as_ref().copied(),
                "precipitation_sum_text": daily.today.precipitation_amount_level().map(|l| l.to_string()),
                "precipitation_hours": daily.today.precipitation_hours.as_ref().copied(),
                "precipitation_probability_max": daily.today.precipitation_probability_max.as_ref().copied(),
                "precipitation_probability_text": daily.today.precipitation_probability_level().map(|l| l.to_string()),
            },
            "tomorrow": {
                "temperature_max": daily.tomorrow.temperature_max.as_ref().copied(),
                "temperature_min": daily.tomorrow.temperature_min.as_ref().copied(),
                "weather_code": daily.tomorrow.weather_code.as_ref().copied(),
                "daylight_duration": daily.tomorrow.daylight_duration.as_ref().copied(),
                "sunshine_duration": daily.tomorrow.sunshine_duration.as_ref().copied(),
                "sunshine_text": daily.tomorrow.sunshine_level().map(|l| l.to_string()),
                "precipitation_sum": daily.tomorrow.precipitation_sum.as_ref().copied(),
                "precipitation_sum_text": daily.tomorrow.precipitation_amount_level().map(|l| l.to_string()),
                "precipitation_hours": daily.tomorrow.precipitation_hours.as_ref().copied(),
                "precipitation_probability_max": daily.tomorrow.precipitation_probability_max.as_ref().copied(),
                "precipitation_probability_text": daily.tomorrow.precipitation_probability_level().map(|l| l.to_string()),
            },
        },
        "air_quality": result.air_quality.as_ref().map(|aq| serde_json::json!({
            "european_aqi": aq.european_aqi.as_ref().copied(),
            "european_aqi_text": aq.air_quality_level().map(|l| l.to_string()),
            "pm10": aq.pm10.as_ref().copied(),
            "pm2_5": aq.pm2_5.as_ref().copied(),
            "pm2_5_text": aq.particulate_matter_level().map(|l| l.to_string()),
        })),
        "voice": {
            "current_summary": current.voice_summary(),
            "today_summary": daily.today.voice_summary(),
            "tomorrow_summary": daily.tomorrow.voice_summary(),
            "air_quality_summary": result.air_quality.as_ref().map(|aq| aq.voice_summary()),
        },
    });
    Ok(json.to_string())
}

fn handle_get_forecast_tool(arguments: &str, default_config: &WeatherServiceConfig) -> Result<String, String> {
    let args: serde_json::Value = serde_json::from_str(arguments).map_err(|e| format!("Invalid arguments: {e}"))?;
    let latitude = args.get("latitude").and_then(|v| v.as_f64()).unwrap_or(default_config.latitude);
    let longitude = args.get("longitude").and_then(|v| v.as_f64()).unwrap_or(default_config.longitude);

    let config = WeatherServiceConfig {
        latitude,
        longitude,
        ..default_config.clone()
    };

    fetch_and_serialize_weather(&config)
}

pub(crate) fn handle_lookup_coordinates_tool(arguments: &str) -> Result<String, String> {
    let args: serde_json::Value = serde_json::from_str(arguments).map_err(|e| format!("Invalid arguments: {e}"))?;
    let place_name = args.get("place_name").and_then(|v| v.as_str()).ok_or("Missing 'place_name' parameter")?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {e}"))?;

    let result = runtime.block_on(async move {
        let client = reqwest::Client::new();
        let response = client
            .get(OPEN_METEO_GEOCODING_URL)
            .query(&[("name", place_name), ("count", "1")])
            .send()
            .await
            .map_err(|e| format!("Geocoding request failed: {e}"))?;
        let json: serde_json::Value = response.json().await.map_err(|e| format!("Failed to parse geocoding response: {e}"))?;
        Ok::<serde_json::Value, String>(json)
    })?;

    let results = result.get("results").and_then(|r| r.as_array()).ok_or("No geocoding results")?;
    let first = results.first().ok_or("No matching location found")?;
    let latitude = first.get("latitude").and_then(|v| v.as_f64()).ok_or("Missing latitude in geocoding result")?;
    let longitude = first.get("longitude").and_then(|v| v.as_f64()).ok_or("Missing longitude in geocoding result")?;
    let name = first.get("name").and_then(|v| v.as_str()).unwrap_or(place_name);
    let country = first.get("country").and_then(|v| v.as_str()).unwrap_or("");

    let json = serde_json::json!({
        "place_name": name,
        "country": country,
        "latitude": latitude,
        "longitude": longitude,
    });
    Ok(json.to_string())
}

pub(crate) fn handle_lookup_location_name_tool(arguments: &str) -> Result<String, String> {
    let args: serde_json::Value = serde_json::from_str(arguments).map_err(|e| format!("Invalid arguments: {e}"))?;
    let latitude = args.get("latitude").and_then(|v| v.as_f64()).ok_or("Missing 'latitude' parameter")?;
    let longitude = args.get("longitude").and_then(|v| v.as_f64()).ok_or("Missing 'longitude' parameter")?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {e}"))?;

    let result = runtime.block_on(async move {
        let client = reqwest::Client::builder()
            .user_agent("smearor-weather-service/0.1.0")
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {e}"))?;
        let url = format!("{NOMINATIM_REVERSE_URL}?lat={latitude}&lon={longitude}&format=json");
        let response = client.get(url).send().await.map_err(|e| format!("Reverse geocoding request failed: {e}"))?;
        let json: serde_json::Value = response.json().await.map_err(|e| format!("Failed to parse reverse geocoding response: {e}"))?;
        Ok::<serde_json::Value, String>(json)
    })?;

    let display_name = result
        .get("display_name")
        .and_then(|v| v.as_str())
        .ok_or("No display name in reverse geocoding response")?;
    let json = serde_json::json!({
        "location_name": display_name,
        "latitude": latitude,
        "longitude": longitude,
    });
    Ok(json.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> WeatherServiceConfig {
        WeatherServiceConfig {
            latitude: 52.52,
            longitude: 13.41,
            ..Default::default()
        }
    }

    #[test]
    fn handle_get_forecast_missing_latitude_uses_default_location() {
        let result = handle_get_forecast_tool(r#"{"longitude": 9.7}"#, &default_config());
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("\"latitude\":52.52"));
        assert!(json.contains("\"longitude\":9.7"));
    }
}
