use crate::config::WeatherWidgetConfig;
use crate::labels::WeatherLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Box as GtkBox;
use gtk4::Image;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::prelude::BoxExt;
use gtk4::prelude::WidgetExt;
use gtk4::prelude::*;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::DefaultFallback;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::GestureHandler;
use smearor_swipe_launcher_plugin_api::GestureHandlersConfiguration;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WidgetBuilder;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;
use smearor_swipe_launcher_plugin_api::WidgetPlugin;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::apply_widget_css_classes;
use smearor_swipe_launcher_plugin_api::apply_widget_scaled_css;
use smearor_swipe_launcher_plugin_api::build_content_box;
use smearor_swipe_launcher_plugin_api::build_spacer_scaled;
use smearor_swipe_launcher_plugin_api::resolve_gtk_nerd_icon;
use smearor_swipe_launcher_plugin_api::sanitize_scale;
use smearor_weather_model::TOPIC_STATUS;
use smearor_weather_model::WeatherCode;
use smearor_weather_model::WeatherCommandMessage;
use smearor_weather_model::WeatherStatusMessage;
use smearor_weather_model::WeatherView;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

pub struct WeatherWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: WeatherWidgetConfig,
    pub icon_image: Rc<RefCell<Option<Image>>>,
    pub temp_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub current_view: Rc<RefCell<usize>>,
    pub latest_status: Rc<RefCell<Option<WeatherStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl WeatherWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: WeatherWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget = WeatherWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            icon_image: Rc::new(RefCell::new(None)),
            temp_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            current_view: Rc::new(RefCell::new(0)),
            latest_status: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_initial_status();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_initial_status(&self) {
        self.get_broadcaster().broadcast_message_to_topic(WeatherCommandMessage::refresh());
    }

    fn request_personalization_status(&self) {
        self.get_broadcaster()
            .broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }
}

impl DefaultFallback for WeatherWidget {
    fn default_fallback(&self, kind: &smearor_swipe_launcher_plugin_api::ActionKind, broadcaster: &MessageBroadcasterInner) {
        use smearor_swipe_launcher_plugin_api::ActionKind;
        match kind {
            ActionKind::Click | ActionKind::DoublePress | ActionKind::SwipeUp | ActionKind::ScrollUp => {
                self.next_view();
                self.broadcast_widget_update();
            }
            ActionKind::Longpress => {
                self.toggle_view();
            }
            ActionKind::RightClick => {
                broadcaster.broadcast_message_to_topic(WeatherCommandMessage::refresh());
            }
            ActionKind::SwipeDown | ActionKind::ScrollDown | ActionKind::MiddleClick => {
                self.prev_view();
                self.broadcast_widget_update();
            }
            ActionKind::Hold | ActionKind::CompoundLongpress | ActionKind::Init => {}
            ActionKind::Expand => {
                self.expand_view();
            }
            ActionKind::Collapse => {
                self.collapse_view();
            }
            ActionKind::ToggleView => {
                self.toggle_view();
            }
        }
    }
}

impl WeatherWidget {
    /// Broadcast a WidgetUpdateMessage so headless/Web instances re-render this widget.
    /// The instance_id is left empty because the widget can't know it (meta.id
    /// doesn't include the instance prefix). The host's route_message derives
    /// the correct instance from the envelope sender_id.
    fn broadcast_widget_update(&self) {
        let plugin_id = self.meta.id.to_string();
        let msg = WidgetUpdateMessage::new(&plugin_id, "");
        let broadcaster = self.get_broadcaster();
        broadcaster.broadcast_message_to_topic(msg);
    }

    fn update_ui(&self, status: &WeatherStatusMessage) {
        let view_index = *self.current_view.borrow();
        let view = self.config.views.get(view_index).copied().unwrap_or(WeatherView::Current);
        let override_data = self.personalization.borrow().clone();

        let view_data = render_view(status, view, &override_data);

        set_weather_icon(&self.icon_image, &view_data.icon_name, self.config.icon_config.icon_size());
        apply_icon_color(&self.icon_image, view_data.icon_color);
        if let Some(ref label) = *self.temp_label.borrow() {
            label.set_text(&view_data.main_text);
        }
        if let Some(ref label) = *self.info_label.borrow() {
            label.set_text(&view_data.info_text);
        }
    }

    fn next_view(&self) {
        if self.config.views.is_empty() {
            return;
        }
        let mut idx = self.current_view.borrow_mut();
        *idx = (*idx + 1) % self.config.views.len();
        let view = self.config.views[*idx];
        drop(idx);

        if let Some(ref status) = *self.latest_status.borrow() {
            let override_data = self.personalization.borrow().clone();
            let view_data = render_view(status, view, &override_data);
            set_weather_icon(&self.icon_image, &view_data.icon_name, self.config.icon_config.icon_size());
            apply_icon_color(&self.icon_image, view_data.icon_color);
            if let Some(ref label) = *self.temp_label.borrow() {
                label.set_text(&view_data.main_text);
            }
            if let Some(ref label) = *self.info_label.borrow() {
                label.set_text(&view_data.info_text);
            }
        }
    }

    fn prev_view(&self) {
        if self.config.views.is_empty() {
            return;
        }
        let mut idx = self.current_view.borrow_mut();
        if *idx == 0 {
            *idx = self.config.views.len() - 1;
        } else {
            *idx -= 1;
        }
        let view = self.config.views[*idx];
        drop(idx);

        if let Some(ref status) = *self.latest_status.borrow() {
            let override_data = self.personalization.borrow().clone();
            let view_data = render_view(status, view, &override_data);
            set_weather_icon(&self.icon_image, &view_data.icon_name, self.config.icon_config.icon_size());
            apply_icon_color(&self.icon_image, view_data.icon_color);
            if let Some(ref label) = *self.temp_label.borrow() {
                label.set_text(&view_data.main_text);
            }
            if let Some(ref label) = *self.info_label.borrow() {
                label.set_text(&view_data.info_text);
            }
        }
    }

    /// Sets the current view to the given index and updates the UI.
    ///
    /// Broadcasts a `WidgetUpdateMessage` so headless and web instances re-render.
    pub(crate) fn set_view_index(&self, index: usize) {
        if self.config.views.is_empty() {
            return;
        }
        let max_index = self.config.views.len() - 1;
        let target = index.min(max_index);
        let mut current = self.current_view.borrow_mut();
        if *current == target {
            return;
        }
        *current = target;
        let view = self.config.views[target];
        drop(current);

        if let Some(ref status) = *self.latest_status.borrow() {
            let override_data = self.personalization.borrow().clone();
            let view_data = render_view(status, view, &override_data);
            set_weather_icon(&self.icon_image, &view_data.icon_name, self.config.icon_config.icon_size());
            apply_icon_color(&self.icon_image, view_data.icon_color);
            if let Some(ref label) = *self.temp_label.borrow() {
                label.set_text(&view_data.main_text);
            }
            if let Some(ref label) = *self.info_label.borrow() {
                label.set_text(&view_data.info_text);
            }
        }
        self.broadcast_widget_update();
    }

    /// Finds the index of the first forecast-type view in the config views list.
    ///
    /// Returns `1` (the typical second entry) if no forecast view is found,
    /// or `0` if the config has only one view.
    fn forecast_view_index(&self) -> usize {
        if self.config.views.len() <= 1 {
            return 0;
        }
        for (i, view) in self.config.views.iter().enumerate() {
            if matches!(view, WeatherView::ForecastToday | WeatherView::ForecastTomorrow) {
                return i;
            }
        }
        1
    }

    /// Toggles between the compact view (index 0) and the forecast view.
    pub(crate) fn toggle_view(&self) {
        let current = *self.current_view.borrow();
        if current == 0 {
            self.set_view_index(self.forecast_view_index());
        } else {
            self.set_view_index(0);
        }
    }

    /// Expands to the forecast view.
    pub(crate) fn expand_view(&self) {
        self.set_view_index(self.forecast_view_index());
    }

    /// Collapses to the compact (first) view.
    pub(crate) fn collapse_view(&self) {
        self.set_view_index(0);
    }
}

/// All CSS classes that can be applied for icon coloring.
const ICON_COLOR_CLASSES: &[&str] = &[
    "icon-color-green",
    "icon-color-light-green",
    "icon-color-yellow",
    "icon-color-orange",
    "icon-color-red",
    "icon-color-dark-red",
    "icon-color-dark-blue",
    "icon-color-blue",
    "icon-color-light-blue",
    "icon-color-black",
    "icon-color-white",
    "icon-color-default",
];

/// Removes all icon color CSS classes, then applies the one for `color` if present.
fn apply_icon_color(icon_image: &Rc<RefCell<Option<Image>>>, color: Option<Color>) {
    if let Some(ref image) = *icon_image.borrow() {
        for class in ICON_COLOR_CLASSES {
            image.remove_css_class(class);
        }
        if let Some(c) = color {
            image.add_css_class(c.css_class());
        }
    }
}

pub(crate) fn render_view(status: &WeatherStatusMessage, view: WeatherView, override_data: &PersonalizationOverride) -> ViewData {
    let locale = override_data.locale;

    if status.is_stale {
        let error = status.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Stale data".to_string());
        return ViewData::error("nf-weather-alien".to_string(), format!("Stale: {error}"));
    }

    if !status.success {
        let error = status.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "No data".to_string());
        return ViewData::error("nf-weather-alien".to_string(), error);
    }

    match view {
        WeatherView::Current => {
            let temp = status
                .current
                .temperature
                .as_ref()
                .copied()
                .map(|t| override_data.format_temperature(t))
                .unwrap_or_else(|| "--".to_string());
            let code = WeatherCode::from_code(status.current.weather_code.as_ref().copied().unwrap_or(999));
            let is_day = status.current.is_day.as_ref().copied().unwrap_or(true);
            let icon = code.icon_day_night(is_day).to_string();
            let desc = code.description().to_string();
            let color = status.current.temperature_level().and_then(|l| l.get_icon_color());
            ViewData::with_color(icon, temp, desc, color)
        }
        WeatherView::ForecastToday => {
            let max = status
                .daily
                .today
                .temperature_max
                .as_ref()
                .copied()
                .map(|t| override_data.format_temperature_with_unit(t))
                .unwrap_or_else(|| "--".to_string());
            let min = status
                .daily
                .today
                .temperature_min
                .as_ref()
                .copied()
                .map(|t| override_data.format_temperature_with_unit(t))
                .unwrap_or_else(|| "--".to_string());
            let code = WeatherCode::from_code(status.daily.today.weather_code.as_ref().copied().unwrap_or(999));
            let icon = code.icon_day_night(true).to_string();
            let label = WeatherLabel::Today.localized_label(locale).to_string();
            ViewData::new(icon, format!("{max} / {min}"), label)
        }
        WeatherView::ForecastTomorrow => {
            let max = status
                .daily
                .tomorrow
                .temperature_max
                .as_ref()
                .copied()
                .map(|t| override_data.format_temperature_with_unit(t))
                .unwrap_or_else(|| "--".to_string());
            let min = status
                .daily
                .tomorrow
                .temperature_min
                .as_ref()
                .copied()
                .map(|t| override_data.format_temperature_with_unit(t))
                .unwrap_or_else(|| "--".to_string());
            let code = WeatherCode::from_code(status.daily.tomorrow.weather_code.as_ref().copied().unwrap_or(999));
            let icon = code.icon_day_night(true).to_string();
            let label = WeatherLabel::Tomorrow.localized_label(locale).to_string();
            ViewData::new(icon, format!("{max} / {min}"), label)
        }
        WeatherView::Wind => {
            let speed = status
                .current
                .wind_speed
                .as_ref()
                .copied()
                .map(|s| override_data.format_wind_speed(s))
                .unwrap_or_else(|| "--".to_string());
            let dir = status
                .current
                .wind_direction()
                .map(|d| d.abbreviation().to_string())
                .unwrap_or_else(|| "".to_string());
            let wind_dir = status.current.wind_direction();
            let icon = wind_dir.and_then(|d| d.get_icon_name()).unwrap_or_else(|| "nf-weather-windy".to_string());
            let color = status.current.wind_speed_level().and_then(|l| l.get_icon_color());
            ViewData::with_color(icon, speed, dir, color)
        }
        WeatherView::Humidity => {
            let humidity = status
                .current
                .relative_humidity
                .as_ref()
                .copied()
                .map(|h| format!("{:.0}%", h))
                .unwrap_or_else(|| "--".to_string());
            let level = status.current.humidity_level();
            let color = level.and_then(|l| l.get_icon_color());
            let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-humidity".to_string());
            let label = WeatherLabel::Humidity.localized_label(locale).to_string();
            ViewData::with_color(icon, humidity, label, color)
        }
        WeatherView::Sunrise => {
            let sunrise = status
                .daily
                .today
                .sunrise
                .as_ref()
                .map(|s| override_data.format_time(s))
                .unwrap_or_else(|| "--:--".to_string());
            let label = WeatherLabel::Sunrise.localized_label(locale).to_string();
            ViewData::new("nf-weather-sunrise".to_string(), sunrise, label)
        }
        WeatherView::Sunset => {
            let sunset = status
                .daily
                .today
                .sunset
                .as_ref()
                .map(|s| override_data.format_time(s))
                .unwrap_or_else(|| "--:--".to_string());
            let label = WeatherLabel::Sunset.localized_label(locale).to_string();
            ViewData::new("nf-weather-sunset".to_string(), sunset, label)
        }
        WeatherView::AirPollution => {
            if let Some(aq) = status.air_quality.as_ref() {
                let aqi = aq
                    .european_aqi
                    .as_ref()
                    .copied()
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "--".to_string());
                let pm = aq.pm2_5.as_ref().copied().map(|v| format!("PM2.5: {:.1}", v)).unwrap_or_else(|| "".to_string());
                let level = aq.air_quality_level();
                let color = level.and_then(|l| l.get_icon_color());
                let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-smog".to_string());
                ViewData::with_color(icon, format!("AQI: {aqi}"), pm, color)
            } else {
                ViewData::new("nf-weather-smog".to_string(), "--".to_string(), "N/A".to_string())
            }
        }
        WeatherView::Pressure => {
            let pressure = status
                .current
                .pressure
                .as_ref()
                .copied()
                .map(|p| override_data.format_pressure(p))
                .unwrap_or_else(|| "--".to_string());
            let level = status.current.pressure_level();
            let color = level.and_then(|l| l.get_icon_color());
            let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-barometer".to_string());
            let label = WeatherLabel::Pressure.localized_label(locale).to_string();
            ViewData::with_color(icon, pressure, label, color)
        }
        WeatherView::UvIndex => {
            let uv = status
                .current
                .uv_index
                .as_ref()
                .copied()
                .map(|v| format!("{:.1}", v))
                .unwrap_or_else(|| "--".to_string());
            let level = status.current.uv_index_level();
            let color = level.and_then(|l| l.get_icon_color());
            let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-day_sunny".to_string());
            let label = WeatherLabel::UvIndex.localized_label(locale).to_string();
            ViewData::with_color(icon, uv, label, color)
        }
        WeatherView::CloudCover => {
            let cover = status
                .current
                .cloud_cover
                .as_ref()
                .copied()
                .map(|c| format!("{:.0}%", c))
                .unwrap_or_else(|| "--".to_string());
            let level = status.current.cloud_cover_level();
            let desc = level.map(|l| l.to_string()).unwrap_or_else(|| "".to_string());
            let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-cloudy".to_string());
            ViewData::new(icon, cover, desc)
        }
        WeatherView::Sunshine => {
            let hours = status
                .daily
                .today
                .sunshine_duration
                .as_ref()
                .copied()
                .map(|s| format!("{:.1}h", s / 3600.0))
                .unwrap_or_else(|| "--".to_string());
            let level = status.daily.today.sunshine_level();
            let label = WeatherLabel::Sunshine.localized_label(locale);
            let desc = level.as_ref().map(|l| l.to_string()).unwrap_or_else(|| label.to_string());
            let color = level.as_ref().and_then(|l| l.get_icon_color());
            let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-day-sunny".to_string());
            ViewData::with_color(icon, hours, desc, color)
        }
        WeatherView::PrecipitationProbability => {
            let prob = status
                .daily
                .today
                .precipitation_probability_max
                .as_ref()
                .copied()
                .map(|p| format!("{:.0}%", p))
                .unwrap_or_else(|| "--".to_string());
            let level = status.daily.today.precipitation_probability_level();
            let label = WeatherLabel::RainChance.localized_label(locale);
            let desc = level.map(|l| l.to_string()).unwrap_or_else(|| label.to_string());
            let color = level.and_then(|l| l.get_icon_color());
            let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-rain-mix".to_string());
            ViewData::with_color(icon, prob, desc, color)
        }
        WeatherView::PrecipitationAmount => {
            let sum = status
                .daily
                .today
                .precipitation_sum
                .as_ref()
                .copied()
                .map(|s| override_data.format_precipitation(s))
                .unwrap_or_else(|| "--".to_string());
            let level = status.daily.today.precipitation_amount_level();
            let label = WeatherLabel::RainAmount.localized_label(locale);
            let desc = level.map(|l| l.to_string()).unwrap_or_else(|| label.to_string());
            let color = level.and_then(|l| l.get_icon_color());
            let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-rain".to_string());
            ViewData::with_color(icon, sum, desc, color)
        }
        WeatherView::Precipitation => {
            let precip = status
                .current
                .precipitation
                .as_ref()
                .copied()
                .map(|p| override_data.format_precipitation(p))
                .unwrap_or_else(|| "--".to_string());
            let level = status.current.precipitation_intensity();
            let label = WeatherLabel::Precipitation.localized_label(locale);
            let desc = level.map(|l| l.to_string()).unwrap_or_else(|| label.to_string());
            let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-rain".to_string());
            ViewData::new(icon, precip, desc)
        }
    }
}

/// Set the weather icon on an Image widget by resolving the icon name
/// to a GTK icon name for symbolic icon recoloring support.
fn set_weather_icon(icon_image: &Rc<RefCell<Option<Image>>>, icon_name: &str, icon_size: i32) {
    if let Some(ref image) = *icon_image.borrow() {
        if let Some(gtk_icon_name) = resolve_gtk_nerd_icon(icon_name) {
            image.set_icon_name(Some(&gtk_icon_name));
        }
        image.set_pixel_size(icon_size);
    }
}

impl MessageHandler<WeatherStatusMessage> for WeatherWidget {
    fn handle_message(&self, message: WeatherStatusMessage, _sender_id: &str) {
        *self.latest_status.borrow_mut() = Some(message.clone());
        self.update_ui(&message);
        self.broadcast_widget_update();
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for WeatherWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("weather widget: received personalization status");
        let status = message.0;
        let locale = status.locale.as_ref().map(|l| Locale::from_str(l).unwrap_or_default()).unwrap_or_default();
        let override_data = PersonalizationOverride {
            temperature_unit: Some(status.temperature_unit),
            wind_speed_unit: Some(status.wind_speed_unit),
            measurement_system: Some(status.measurement_system),
            time_format: Some(status.time_format),
            locale,
        };
        *self.personalization.borrow_mut() = override_data;
        if let Some(ref status) = *self.latest_status.borrow() {
            self.update_ui(status);
        }
    }
}

impl AcceptTopic<FfiEnvelope> for WeatherWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_STATUS || topic == TOPIC_PERSONALIZATION_STATUS || topic == TOPIC_MCP_INVOKE_TOOL || topic == TOPIC_MCP_INVOKE_RESOURCE
    }
}

impl MessageBroadcaster for WeatherWidget {}

impl PluginMetaGetter for WeatherWidget {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WeatherWidget {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl WidgetPlugin for WeatherWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                trace!("weather widget: on_message topic={} type_id={}", topic, envelope.type_id);
                if envelope.type_id == WeatherStatusMessage::TYPE_ID {
                    MessageHandler::<WeatherStatusMessage>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<PersonalizationStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl WidgetBuilder for WeatherWidget {
    fn build_widget(&mut self) -> Widget {
        let scale = sanitize_scale(self.config.dimensions.scale.unwrap_or(1.0));
        let outer_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(self.config.layout.spacing_scaled(scale))
            .css_classes(["scroll-item", "menu-button", "weather-widget"])
            .vexpand(true)
            .build();

        outer_box.set_width_request(self.config.dimensions.width_scaled(scale));
        outer_box.set_height_request(self.config.dimensions.height_scaled(scale));

        let inner_box = build_content_box(self.config.layout.spacing_scaled(scale), &["menu_button_inner"]);

        let icon_image = Image::builder().css_classes(["weather-icon", "nerd-icon"]).build();
        icon_image.set_pixel_size(self.config.icon_config.icon_size_scaled(scale));
        set_weather_icon(&self.icon_image, "nf-weather-alien", self.config.icon_config.icon_size_scaled(scale));
        let temp_label = Label::builder().css_classes(["widget-main-text".to_string()]).build();
        let info_label = Label::builder().css_classes(["widget-info-text".to_string()]).build();

        temp_label.set_text("--");
        info_label.set_text("Loading...");

        temp_label.set_height_request((20.0 * scale).round() as i32);
        apply_text_color(&temp_label, self.config.text_colors.main_text_color());
        info_label.set_height_request((16.0 * scale).round() as i32);
        apply_text_color(&info_label, self.config.text_colors.info_text_color());

        inner_box.append(&icon_image);
        if !self.config.icon_config.icon_only() {
            inner_box.append(&temp_label);
            inner_box.append(&info_label);
        }

        let spacer = build_spacer_scaled(16, scale);
        inner_box.append(&spacer);

        outer_box.append(&inner_box);

        *self.icon_image.borrow_mut() = Some(icon_image.clone());
        *self.temp_label.borrow_mut() = Some(temp_label.clone());
        *self.info_label.borrow_mut() = Some(info_label.clone());

        let widget_self = Rc::new(Self {
            meta: self.meta.clone(),
            core_context: self.core_context,
            config: self.config.clone(),
            icon_image: self.icon_image.clone(),
            temp_label: self.temp_label.clone(),
            info_label: self.info_label.clone(),
            current_view: self.current_view.clone(),
            latest_status: self.latest_status.clone(),
            personalization: self.personalization.clone(),
        });

        let outer_widget = outer_box.upcast::<Widget>();
        apply_widget_css_classes(&outer_widget, &self.meta.id, &self.config.layout.css_classes);
        if scale != 1.0 {
            apply_widget_scaled_css(&outer_widget, scale);
        }
        let message_broadcaster = self.get_broadcaster();
        widget_self.attach_gesture_handlers(&outer_widget, &self.config.actions, &message_broadcaster, &GestureHandlersConfiguration::default());

        outer_widget
    }
}
