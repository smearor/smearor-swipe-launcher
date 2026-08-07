use crate::command::PersonalizationCommand;
use crate::config::PersonalizationServiceConfig;
use crate::portal;
use crate::state::LatestPersonalizationState;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT;
use smearor_personalization_model::ColorScheme;
use smearor_personalization_model::DateFormat;
use smearor_personalization_model::FirstDayOfWeek;
use smearor_personalization_model::GeoCoordinates;
use smearor_personalization_model::MeasurementSystem;
use smearor_personalization_model::PersonalizationCommandAction;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TemperatureUnit;
use smearor_personalization_model::TimeFormat;
use smearor_personalization_model::WindSpeedUnit;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::FromLocale;
use smearor_swipe_launcher_plugin_api::Locale;
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
use smearor_swipe_launcher_plugin_api::box_payload;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::error;
use tracing::trace;

pub struct PersonalizationService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    #[allow(unused)]
    pub config: PersonalizationServiceConfig,
    pub command_sender: tokio::sync::mpsc::UnboundedSender<PersonalizationCommand>,
    pub latest_state: Arc<RwLock<LatestPersonalizationState>>,
}

impl PersonalizationService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_personalization_model::register_json_converters(core_context);

        let service_config: PersonalizationServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel::<PersonalizationCommand>();
        let meta = PluginMeta::try_from(&config)?;
        let meta_clone = meta.clone();
        let core_context_clone = core_context;
        let service_config_clone = service_config.clone();
        let latest_state = Arc::new(RwLock::new(LatestPersonalizationState::default()));
        let latest_state_clone = latest_state.clone();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    error!("Personalization service: failed to create tokio runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_update_loop(service_config_clone, command_receiver, meta_clone, core_context_clone, latest_state_clone).await;
            });
        });

        let service = PersonalizationService {
            meta,
            core_context,
            config: service_config,
            command_sender,
            latest_state,
        };
        service.register_mcp_capabilities();
        Ok(service)
    }

    pub(crate) fn send_response<T: TypedMessage + SharedMessage + Clone>(&self, message: T, sender_id: &str) {
        let payload_ptr = box_payload(message.clone());
        let sender_id_string = sender_id.to_string();
        let topic = message.topic();
        trace!("personalization: send_response topic={} to sender_id={}", topic, sender_id);
        let envelope = FfiEnvelope::builder()
            .sender_id(self.meta.id.clone())
            .target_instance_id(sender_id_string.as_str())
            .topic(topic)
            .type_id(T::TYPE_ID)
            .payload(payload_ptr)
            .destroy_payload(Some(default_destroy_payload))
            .clone_payload(Some(default_clone_payload::<T>))
            .build();
        if let Some(context) = &self.core_context {
            context.send_message(envelope);
        } else {
            trace!("personalization: no core_context, cannot send response");
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationCommandMessage>> for PersonalizationService {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationCommandMessage>, _sender_id: &str) {
        trace!("Personalization service: received command {:?}", message.action);
        match &message.action {
            PersonalizationCommandAction::Refresh => {
                let _ = self.command_sender.send(PersonalizationCommand::Refresh);
            }
            PersonalizationCommandAction::UpdateLocation(coordinates) => {
                let _ = self.command_sender.send(PersonalizationCommand::UpdateLocation(coordinates.clone()));
            }
            PersonalizationCommandAction::UpdateLocale(locale) => {
                let locale_string = locale.to_string();
                let _ = self.command_sender.send(PersonalizationCommand::UpdateLocale(locale_string));
            }
            PersonalizationCommandAction::RequestStatus => {
                let _ = self.command_sender.send(PersonalizationCommand::RequestStatus);
            }
        }
    }
}

impl MessageBroadcaster for PersonalizationService {}

impl MessageTopicBroadcaster<PersonalizationStatusMessage> for PersonalizationService {}

impl PluginMetaGetter for PersonalizationService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for PersonalizationService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl ServicePlugin for PersonalizationService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                trace!("personalization: on_message topic={} type_id={}", topic, envelope.type_id);
                if envelope.type_id == FfiEnvelopePayload::<PersonalizationCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationCommandMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_PROMPT && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokePromptMessage>::TYPE_ID {
                    trace!("personalization: received InvokePromptMessage, no prompts registered, ignoring");
                } else {
                    trace!("personalization: unknown type_id");
                }
            }
        }
    }
}

async fn run_update_loop(
    config: PersonalizationServiceConfig,
    mut command_receiver: tokio::sync::mpsc::UnboundedReceiver<PersonalizationCommand>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
    latest_state: Arc<RwLock<LatestPersonalizationState>>,
) {
    let interval_seconds = config.update_interval_seconds.max(60);
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let location_interval_seconds = config.location_update_interval_seconds.max(300);
    let location_interval_dur = tokio::time::Duration::from_secs(location_interval_seconds);

    // Runtime overrides
    let mut runtime_location_override: Option<GeoCoordinates> = None;
    let mut runtime_locale_override: Option<String> = None;

    // Cached portal data (queried separately from the main update interval)
    let mut cached_portal_coords: Option<GeoCoordinates> = None;
    let mut cached_color_scheme: Option<ColorScheme> = None;
    let mut force_location_query = true;
    let mut force_color_scheme_query = true;
    let mut last_location_query = tokio::time::Instant::now();
    let mut last_color_scheme_query = tokio::time::Instant::now();

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            command = command_receiver.recv() => {
                match command {
                    None => break,
                    Some(PersonalizationCommand::Refresh) => {
                        trace!("personalization: clearing runtime overrides and refreshing");
                        runtime_location_override = None;
                        runtime_locale_override = None;
                        cached_portal_coords = None;
                        cached_color_scheme = None;
                        force_location_query = true;
                        force_color_scheme_query = true;
                    }
                    Some(PersonalizationCommand::UpdateLocation(coords)) => {
                        trace!("personalization: runtime location override set");
                        runtime_location_override = Some(coords);
                    }
                    Some(PersonalizationCommand::UpdateLocale(locale)) => {
                        trace!("personalization: runtime locale override set: {}", locale);
                        runtime_locale_override = Some(locale);
                    }
                    Some(PersonalizationCommand::RequestStatus) => {
                        trace!("personalization: request status re-broadcast");
                    }
                }
            }
        }

        // Compute config coordinates (immutable, from config)
        let config_coords = if let (Some(lat), Some(lon)) = (config.latitude, config.longitude) {
            Some(GeoCoordinates {
                latitude: lat,
                longitude: lon,
                location_name: config
                    .location_name
                    .clone()
                    .map(stabby::string::String::from)
                    .map(stabby::option::Option::Some)
                    .unwrap_or(stabby::option::Option::None()),
            })
        } else {
            None
        };

        // Query portal location only when needed: on startup, after refresh, or when the location interval elapses
        if runtime_location_override.is_none() && config_coords.is_none() && config.enable_location {
            let should_query = force_location_query || last_location_query.elapsed() >= location_interval_dur;
            if should_query {
                if let Some(mut new_coords) = portal::query_location().await {
                    let is_significant_change = cached_portal_coords.as_ref().map_or(true, |cached| {
                        (new_coords.latitude - cached.latitude).abs() > config.location_change_threshold
                            || (new_coords.longitude - cached.longitude).abs() > config.location_change_threshold
                    });
                    if is_significant_change {
                        trace!("personalization: location changed significantly, updating cached coords");
                        if new_coords.location_name.is_none() {
                            if let Some(name) = portal::reverse_geocode(new_coords.latitude, new_coords.longitude).await {
                                new_coords.location_name = stabby::option::Option::Some(stabby::string::String::from(name));
                            }
                        }
                        cached_portal_coords = Some(new_coords);
                    } else {
                        trace!("personalization: location change below threshold, keeping cached coords");
                    }
                }
                force_location_query = false;
                last_location_query = tokio::time::Instant::now();
            }
        }

        // Compute effective coordinates from all sources (after portal query may have updated cache)
        let mut effective_coords = runtime_location_override.clone().or(config_coords).or(cached_portal_coords.clone());

        // Reverse geocode config/runtime coords if not yet done and location interval elapsed
        if let Some(ref mut coords) = effective_coords {
            if coords.location_name.is_none() && (force_location_query || last_location_query.elapsed() >= location_interval_dur) {
                if let Some(name) = portal::reverse_geocode(coords.latitude, coords.longitude).await {
                    coords.location_name = stabby::option::Option::Some(stabby::string::String::from(name));
                }
                force_location_query = false;
                last_location_query = tokio::time::Instant::now();
            }
        }

        // Query color scheme on main interval or when forced
        if config.color_scheme.is_none() {
            let should_query = force_color_scheme_query || last_color_scheme_query.elapsed() >= tokio::time::Duration::from_secs(interval_seconds);
            if should_query {
                cached_color_scheme = portal::query_color_scheme().await;
                force_color_scheme_query = false;
                last_color_scheme_query = tokio::time::Instant::now();
            }
        }

        let status = build_status(&config, &runtime_location_override, &runtime_locale_override, &effective_coords, &cached_color_scheme);

        {
            if let Ok(mut guard) = latest_state.write() {
                guard.status = status.clone();
            }
        }

        broadcast(&meta, &core_context, status);
        trace!("personalization: broadcasted status");
    }
}

fn build_status(
    config: &PersonalizationServiceConfig,
    _runtime_location_override: &Option<GeoCoordinates>,
    runtime_locale_override: &Option<String>,
    resolved_coords: &Option<GeoCoordinates>,
    cached_color_scheme: &Option<ColorScheme>,
) -> PersonalizationStatusMessage {
    // Query timezone
    let system_timezone = iana_time_zone::get_timezone().ok();
    let timezone = runtime_locale_override
        .as_ref()
        .and(None)
        .or_else(|| config.timezone.clone())
        .or(system_timezone)
        .map(stabby::string::String::from)
        .map(stabby::option::Option::Some)
        .unwrap_or(stabby::option::Option::None());

    // Query locale
    let system_locale = sys_locale::get_locale().map(|l| l.replace('_', "-"));
    let stabby_locale = runtime_locale_override
        .clone()
        .or_else(|| config.locale.clone())
        .or(system_locale.clone())
        .map(stabby::string::String::from)
        .map(stabby::option::Option::Some)
        .unwrap_or(stabby::option::Option::None());

    let coordinates = resolved_coords
        .clone()
        .map(stabby::option::Option::Some)
        .unwrap_or(stabby::option::Option::None());

    // Derive units/formats from config override (FromStr) or locale (from_locale).
    let locale_str = runtime_locale_override
        .clone()
        .or_else(|| config.locale.clone())
        .or(system_locale.clone())
        .unwrap_or_default();

    let locale = Locale::from_str(&locale_str).unwrap_or_default();

    let temperature_unit = config
        .temperature_unit
        .as_deref()
        .map(|s| TemperatureUnit::from_str(s).unwrap_or_default())
        .unwrap_or_else(|| TemperatureUnit::from_locale(locale));

    let wind_speed_unit = config
        .wind_speed_unit
        .as_deref()
        .map(|s| WindSpeedUnit::from_str(s).unwrap_or_default())
        .unwrap_or_else(|| WindSpeedUnit::from_locale(locale));

    let time_format = config
        .time_format
        .as_deref()
        .map(|s| TimeFormat::from_str(s).unwrap_or_default())
        .unwrap_or_else(|| TimeFormat::from_locale(locale));

    let date_format = config
        .date_format
        .as_deref()
        .map(|s| DateFormat::from_str(s).unwrap_or_default())
        .unwrap_or_else(|| DateFormat::from_locale(locale));

    let first_day_of_week = config
        .first_day_of_week
        .as_deref()
        .map(|s| FirstDayOfWeek::from_str(s).unwrap_or_default())
        .unwrap_or_else(|| FirstDayOfWeek::from_locale(locale));

    let measurement_system = config
        .measurement_system
        .as_deref()
        .map(|s| MeasurementSystem::from_str(s).unwrap_or_default())
        .unwrap_or_else(|| MeasurementSystem::from_locale(locale));

    let color_scheme = if let Some(scheme_str) = config.color_scheme.as_deref() {
        ColorScheme::from_str(scheme_str).unwrap_or(ColorScheme::System)
    } else {
        cached_color_scheme.clone().unwrap_or(ColorScheme::System)
    };

    PersonalizationStatusMessage {
        coordinates,
        timezone,
        locale: stabby_locale,
        temperature_unit,
        wind_speed_unit,
        time_format,
        date_format,
        first_day_of_week,
        measurement_system,
        color_scheme,
        success: true,
        error_message: stabby::option::Option::None(),
    }
}

pub(crate) fn parse_coordinates(arguments: &str) -> Result<GeoCoordinates, String> {
    let json: serde_json::Value = serde_json::from_str(arguments).map_err(|e| format!("Invalid JSON: {e}"))?;
    let latitude = json.get("latitude").and_then(|v| v.as_f64()).ok_or("Missing or invalid 'latitude'")?;
    let longitude = json.get("longitude").and_then(|v| v.as_f64()).ok_or("Missing or invalid 'longitude'")?;
    let location_name = json
        .get("location_name")
        .and_then(|v| v.as_str())
        .map(stabby::string::String::from)
        .map(stabby::option::Option::Some)
        .unwrap_or(stabby::option::Option::None());
    Ok(GeoCoordinates {
        latitude,
        longitude,
        location_name,
    })
}

pub(crate) fn parse_locale(arguments: &str) -> Result<String, String> {
    let json: serde_json::Value = serde_json::from_str(arguments).map_err(|e| format!("Invalid JSON: {e}"))?;
    let locale = json.get("locale").and_then(|v| v.as_str()).ok_or("Missing or invalid 'locale'")?;
    Ok(locale.to_string())
}

fn broadcast<T: Clone + MessageTopic + TypedMessage>(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, message: T) {
    let payload_ptr = box_payload(message.clone());
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("")
        .topic(T::topic())
        .type_id(T::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<T>))
        .build();

    if let Some(context) = core_context {
        context.send_message(envelope);
    }
}
