use serde::Deserialize;
use serde_json::Value;
use typed_builder::TypedBuilder;

pub const DEFAULT_FORMAT: &str = "[hour]:[minute]:[second]";

pub const DEFAULT_FORMAT_2: &str = "[day].[month].[year]";

pub const DEFAULT_WIDTH: i32 = 100;

#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct ClockConfig {
    /// The date time format.
    #[builder(default = DEFAULT_FORMAT.to_string())]
    pub(crate) format: String,

    /// The date time format.
    #[builder(default)]
    pub(crate) format_2: Option<String>,

    /// The timezone..
    #[builder(default, setter(into))]
    pub(crate) timezone: Option<String>,

    /// The width of the widget.
    #[builder(default, setter(into))]
    pub(crate) width: Option<i32>,

    /// The background color of the widget.
    #[builder(default, setter(into))]
    pub(crate) background_color: Option<String>,

    /// Message topic for single-click
    #[serde(default)]
    pub click_topic: Option<String>,

    /// Message payload for single-click (JSON/TOML)
    #[serde(default)]
    pub click_payload: Option<Value>,

    /// Message topic for long-press
    #[serde(default)]
    pub longpress_topic: Option<String>,

    /// Message payload for long-press (JSON/TOML)
    #[serde(default)]
    pub longpress_payload: Option<Value>,

    /// Spacing between child widgets inside the clock widget.
    #[serde(default)]
    pub spacing: i32,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: DEFAULT_FORMAT.to_string(),
            format_2: None,
            timezone: None,
            width: None,
            background_color: None,
            click_topic: None,
            click_payload: None,
            longpress_topic: None,
            longpress_payload: None,
            spacing: 0,
        }
    }
}
