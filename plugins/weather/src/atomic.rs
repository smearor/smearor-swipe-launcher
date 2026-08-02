use crate::labels::WeatherLabel;
use crate::personalization::PersonalizationOverride;
use gtk4::Label;
use gtk4::prelude::*;
use smearor_model_widget::AtomicWidgetConfig;
use smearor_personalization_model::PersonalizationCommandMessage;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_render_utils::resolve_icon_codepoint;
use smearor_swipe_launcher_plugin_api::AtomicGraphicData;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::Locale;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::ViewData;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;
use smearor_swipe_launcher_plugin_api::apply_text_color;
use smearor_swipe_launcher_plugin_api::atomic_widget_impl;
use smearor_weather_model::TOPIC_STATUS;
use smearor_weather_model::WeatherCode;
use smearor_weather_model::WeatherCommandMessage;
use smearor_weather_model::WeatherStatusMessage;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use tracing::trace;

/// Which weather view an atomic widget renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicView {
    /// Current conditions: icon + temperature + description.
    Current,
    /// Today's forecast: icon + max/min temps.
    ForecastToday,
    /// Tomorrow's forecast: icon + max/min temps.
    ForecastTomorrow,
    /// UV index.
    UvIndex,
    /// Sunrise time only.
    Sunrise,
    /// Sunset time only.
    Sunset,
    /// Cloud cover percentage and description.
    CloudCover,
    /// Sunshine duration for today.
    Sunshine,
    /// Precipitation probability for today.
    PrecipitationProbability,
    /// Precipitation amount sum for today.
    PrecipitationAmount,
    /// Current precipitation (rain, showers, snowfall).
    Precipitation,
    /// Wind speed and direction.
    Wind,
    /// Relative humidity.
    Humidity,
    /// Air quality index and pollutants.
    AirPollution,
    /// Atmospheric pressure.
    Pressure,
}

impl FromStr for AtomicView {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "weather_today" => Ok(Self::Current),
            "weather_forecast" => Ok(Self::ForecastToday),
            "weather_tomorrow" => Ok(Self::ForecastTomorrow),
            "weather_uv_index" => Ok(Self::UvIndex),
            "weather_sunrise" => Ok(Self::Sunrise),
            "weather_sunset" => Ok(Self::Sunset),
            "weather_cloud_cover" => Ok(Self::CloudCover),
            "weather_sunshine" => Ok(Self::Sunshine),
            "weather_precipitation_probability" => Ok(Self::PrecipitationProbability),
            "weather_precipitation_amount" => Ok(Self::PrecipitationAmount),
            "weather_precipitation" => Ok(Self::Precipitation),
            "weather_wind" => Ok(Self::Wind),
            "weather_humidity" => Ok(Self::Humidity),
            "weather_air_pollution" => Ok(Self::AirPollution),
            "weather_pressure" => Ok(Self::Pressure),
            _ => Err(format!("Unknown weather atomic view: {s}")),
        }
    }
}

impl AtomicView {
    /// Renders this view's display data from the weather status and personalization override.
    pub fn render(&self, status: &WeatherStatusMessage, override_data: &PersonalizationOverride) -> ViewData {
        let locale = override_data.locale;
        if status.is_stale {
            let error = status.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Stale data".to_string());
            return ViewData::error("nf-weather-alien".to_string(), format!("Stale: {error}"));
        }
        if !status.success {
            let error = status.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "No data".to_string());
            return ViewData::error("nf-weather-alien".to_string(), error);
        }
        match self {
            Self::Current => {
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
            Self::ForecastToday => {
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
            Self::ForecastTomorrow => {
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
            Self::UvIndex => {
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
            Self::Sunrise => {
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
            Self::Sunset => {
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
            Self::CloudCover => {
                let cover = status
                    .current
                    .cloud_cover
                    .as_ref()
                    .copied()
                    .map(|c| format!("{:.0}%", c))
                    .unwrap_or_else(|| "--".to_string());
                let level = status.current.cloud_cover_level();
                let desc = level.map(|l| l.to_string()).unwrap_or_default();
                let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-cloudy".to_string());
                ViewData::new(icon, cover, desc)
            }
            Self::Sunshine => {
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
            Self::PrecipitationProbability => {
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
            Self::PrecipitationAmount => {
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
            Self::Precipitation => {
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
            Self::Wind => {
                let speed = status
                    .current
                    .wind_speed
                    .as_ref()
                    .copied()
                    .map(|s| override_data.format_wind_speed(s))
                    .unwrap_or_else(|| "--".to_string());
                let dir = status.current.wind_direction().map(|d| d.abbreviation().to_string()).unwrap_or_default();
                let wind_dir = status.current.wind_direction();
                let icon = wind_dir.and_then(|d| d.get_icon_name()).unwrap_or_else(|| "nf-weather-windy".to_string());
                let color = status.current.wind_speed_level().and_then(|l| l.get_icon_color());
                ViewData::with_color(icon, speed, dir, color)
            }
            Self::Humidity => {
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
            Self::AirPollution => {
                if let Some(aq) = status.air_quality.as_ref() {
                    let aqi = aq
                        .european_aqi
                        .as_ref()
                        .copied()
                        .map(|v| format!("{:.0}", v))
                        .unwrap_or_else(|| "--".to_string());
                    let pm = aq.pm2_5.as_ref().copied().map(|v| format!("PM2.5: {:.1}", v)).unwrap_or_default();
                    let level = aq.air_quality_level();
                    let color = level.and_then(|l| l.get_icon_color());
                    let icon = level.and_then(|l| l.get_icon_name()).unwrap_or_else(|| "nf-weather-smog".to_string());
                    ViewData::with_color(icon, format!("AQI: {aqi}"), pm, color)
                } else {
                    ViewData::new("nf-weather-smog".to_string(), "--".to_string(), "N/A".to_string())
                }
            }
            Self::Pressure => {
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
        }
    }
}

/// Atomic weather widget that renders a single weather view.
///
/// Subscribes to `service.weather.state` and renders only the view specified
/// at construction time. No view switching — each atomic widget is a
/// single-purpose display.
pub struct WeatherAtomicWidget {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AtomicWidgetConfig,
    pub view: AtomicView,
    pub icon_label: Rc<RefCell<Option<Label>>>,
    pub main_label: Rc<RefCell<Option<Label>>>,
    pub info_label: Rc<RefCell<Option<Label>>>,
    pub latest_status: Rc<RefCell<Option<WeatherStatusMessage>>>,
    pub personalization: Rc<RefCell<PersonalizationOverride>>,
}

impl WeatherAtomicWidget {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let widget_config: AtomicWidgetConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let widget_name = config.config.get("widget").and_then(|v| v.as_str()).unwrap_or_default();

        let view = AtomicView::from_str(widget_name).unwrap_or(AtomicView::Current);

        let widget = WeatherAtomicWidget {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            config: widget_config,
            view,
            icon_label: Rc::new(RefCell::new(None)),
            main_label: Rc::new(RefCell::new(None)),
            info_label: Rc::new(RefCell::new(None)),
            latest_status: Rc::new(RefCell::new(None)),
            personalization: Rc::new(RefCell::new(PersonalizationOverride::default())),
        };
        widget.register_mcp_capabilities();
        widget.request_initial_status();
        widget.request_personalization_status();
        Ok(widget)
    }

    fn request_personalization_status(&self) {
        MessageBroadcaster::get_broadcaster(self).broadcast_message_to_topic(PersonalizationCommandMessage::request_status());
    }

    fn update_ui(&self, status: &WeatherStatusMessage) {
        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data).with_text_colors(&self.config.text_colors);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f07b}');
        smearor_swipe_launcher_plugin_api::update_labels(
            &*self.icon_label.borrow(),
            &*self.main_label.borrow(),
            &*self.info_label.borrow(),
            &icon_char.to_string(),
            &view_data.main_text,
            &view_data.info_text,
        );
        apply_atomic_icon_color(&self.icon_label, view_data.icon_color);
        if let Some(ref label) = *self.main_label.borrow() {
            apply_text_color(label, view_data.main_text_color);
        }
        if let Some(ref label) = *self.info_label.borrow() {
            apply_text_color(label, view_data.info_text_color);
        }
    }

    /// Extract graphic rendering data from the latest status.
    ///
    /// Returns `(icon_char, main_text, info_text, is_error)` for the
    /// centralised rendering pipeline.
    fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
        let status = self.latest_status.borrow();
        let Some(status) = status.as_ref() else {
            return AtomicGraphicData::error('\u{f07b}', "Loading...".to_string());
        };

        if status.is_stale {
            let error = status.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "Stale data".to_string());
            return AtomicGraphicData::error('\u{f07b}', format!("Stale: {error}"));
        }

        if !status.success {
            let error = status.error_message.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "No data".to_string());
            return AtomicGraphicData::error('\u{f07b}', error);
        }

        let override_data = self.personalization.borrow().clone();
        let view_data = self.view.render(status, &override_data).with_text_colors(&self.config.text_colors);
        let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f07b}');
        let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
        data.is_error = view_data.is_error;
        data.icon_color = view_data.icon_color.map(|c| c.to_rgba());
        data.main_text_color = view_data.main_text_color.map(|c| c.to_rgba());
        data.info_text_color = view_data.info_text_color.map(|c| c.to_rgba());
        data
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
fn apply_atomic_icon_color(icon_label: &Rc<RefCell<Option<Label>>>, color: Option<Color>) {
    if let Some(ref label) = *icon_label.borrow() {
        for class in ICON_COLOR_CLASSES {
            label.remove_css_class(class);
        }
        if let Some(c) = color {
            label.add_css_class(c.css_class());
        }
    }
}

atomic_widget_impl! {
    widget: WeatherAtomicWidget,
    status: WeatherStatusMessage,
    topic: TOPIC_STATUS,
    debug_tag: "weather-atomic",
    mcp_description: "Weather atomic widget",
    css_prefix: "weather",
    default_icon: '\u{f07b}',
    default_main: "--",
    default_info: "Loading...",
    refresh_command: WeatherCommandMessage::refresh(),
    extra_message_types: [FfiEnvelopePayload<PersonalizationStatusMessage>]
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for WeatherAtomicWidget {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("weather atomic widget: received personalization status");
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
