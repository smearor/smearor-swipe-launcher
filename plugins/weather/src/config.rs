use serde::Deserialize;
use serde_json::Value;
use smearor_weather_model::WeatherView;
use typed_builder::TypedBuilder;

pub const DEFAULT_WIDTH: i32 = 120;
pub const DEFAULT_HEIGHT: i32 = 80;

/// Configuration for the weather widget.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct WeatherWidgetConfig {
    /// The width of the widget in pixels.
    #[builder(default, setter(into))]
    pub(crate) width: Option<i32>,

    /// The height of the widget in pixels.
    #[builder(default, setter(into))]
    pub(crate) height: Option<i32>,

    /// The background color of the widget.
    #[builder(default, setter(into))]
    pub(crate) background_color: Option<String>,

    /// Views to cycle through on click.
    #[builder(default)]
    pub(crate) views: Vec<WeatherView>,

    /// Message topic for single-click.
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click (JSON/TOML).
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Message topic for long-press.
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press (JSON/TOML).
    #[serde(default)]
    pub longpress_payload: Option<Value>,
}

impl Default for WeatherWidgetConfig {
    fn default() -> Self {
        Self {
            width: Some(DEFAULT_WIDTH),
            height: Some(DEFAULT_HEIGHT),
            background_color: None,
            views: vec![WeatherView::Current, WeatherView::ForecastToday, WeatherView::Wind, WeatherView::Humidity],
            click_topic: None,
            click_payload: None,
            longpress_topic: None,
            longpress_payload: None,
        }
    }
}
