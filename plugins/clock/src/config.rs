use serde::Deserialize;
use typed_builder::TypedBuilder;

pub const DEFAULT_FORMAT: &str = "[hour]:[minute]:[second]";

pub const DEFAULT_DESCRIPTION: &str = "Clock";

#[derive(Debug, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct ClockConfig {
    /// The date time format.
    #[builder(default = DEFAULT_FORMAT.to_string())]
    pub(crate) format: String,

    #[builder(default = DEFAULT_DESCRIPTION.to_string())]
    pub(crate) description: String,

    /// The background color of the widget.
    #[builder(default, setter(into))]
    pub(crate) background_color: Option<String>,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: DEFAULT_FORMAT.to_string(),
            description: DEFAULT_DESCRIPTION.to_string(),
            background_color: None,
        }
    }
}
