use crate::widget::Color;
use crate::widget::icons::deserialize_hex_color;
use crate::widget::icons::serialize_hex_color;
use serde::Deserialize;
use serde::Serialize;
use typed_builder::TypedBuilder;

/// Default main_text_color value.
pub const DEFAULT_MAIN_TEXT_COLOR: Option<Color> = None;

/// Default info_text_color value.
pub const DEFAULT_INFO_TEXT_COLOR: Option<Color> = None;

/// Widget text color configuration for GTK, atomic, headless, and web rendering.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// fields `main_text_color` and `info_text_color` map directly to this struct.
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetTextColors {
    /// Optional color for the main text line, parsed from a hex string (e.g. "#ff6600", "#f60", "#ff660080").
    /// Acts as a fallback when no semantic color is provided by the widget.
    #[serde(deserialize_with = "deserialize_hex_color", serialize_with = "serialize_hex_color", default)]
    #[builder(default, setter(into, strip_option))]
    pub main_text_color: Option<Color>,

    /// Optional color for the info text line, parsed from a hex string (e.g. "#ff6600", "#f60", "#ff660080").
    /// Acts as a fallback when no semantic color is provided by the widget.
    #[serde(deserialize_with = "deserialize_hex_color", serialize_with = "serialize_hex_color", default)]
    #[builder(default, setter(into, strip_option))]
    pub info_text_color: Option<Color>,
}

impl Default for WidgetTextColors {
    fn default() -> Self {
        Self {
            main_text_color: DEFAULT_MAIN_TEXT_COLOR,
            info_text_color: DEFAULT_INFO_TEXT_COLOR,
        }
    }
}

impl WidgetTextColors {
    /// Returns the configured main text color, if set.
    pub fn main_text_color(&self) -> Option<Color> {
        self.main_text_color
    }

    /// Returns the configured info text color, if set.
    pub fn info_text_color(&self) -> Option<Color> {
        self.info_text_color
    }
}
