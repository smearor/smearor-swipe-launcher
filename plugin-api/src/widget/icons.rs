use crate::widget::Color;
use serde::Deserialize;
use serde::Serialize;
use std::str::FromStr;
use typed_builder::TypedBuilder;

/// Default icon size in pixels for GTK widgets.
///
/// Widgets that render icons via GTK (`Image::set_pixel_size`, etc.) should use
/// this as the default `icon_size` value. Atomic widgets do **not** use this
/// constant — their icon size is derived from the physical button dimensions to
/// ensure room for `main_text` and `info_text`.
pub const DEFAULT_ICON_SIZE: i32 = 36;

/// Default icon_only value for GTK widgets.
pub const DEFAULT_ICON_ONLY: bool = false;

/// Default icon_color value for GTK widgets.
pub const DEFAULT_ICON_COLOR: Option<Color> = None;

/// Widget icon configuration for GTK layout.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// fields `icon_size`, `icon_only`, and `icon_color` map directly to this struct.
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetIcon {
    /// Icon size in pixels.
    #[builder(default, setter(into))]
    pub icon_size: i32,

    /// Show only the icon without text labels.
    #[builder(default, setter(into))]
    pub icon_only: bool,

    /// Optional icon color, parsed from a hex string (e.g. "#ff6600", "#f60", "#ff660080").
    /// Acts as a fallback when no semantic color is provided by the widget.
    #[serde(deserialize_with = "deserialize_hex_color", serialize_with = "serialize_hex_color", default)]
    #[builder(default, setter(into, strip_option))]
    pub icon_color: Option<Color>,
}

impl Default for WidgetIcon {
    fn default() -> Self {
        Self {
            icon_size: DEFAULT_ICON_SIZE,
            icon_only: DEFAULT_ICON_ONLY,
            icon_color: DEFAULT_ICON_COLOR,
        }
    }
}

impl WidgetIcon {
    /// Returns the icon size.
    pub fn icon_size(&self) -> i32 {
        self.icon_size
    }

    /// Returns whether only the icon should be shown.
    pub fn icon_only(&self) -> bool {
        self.icon_only
    }

    /// Returns the configured icon color, if set.
    pub fn icon_color(&self) -> Option<Color> {
        self.icon_color
    }
}

/// Deserializes an optional hex color string into `Option<Color>`.
///
/// Accepts a TOML string value like `"#ff6600"` and parses it via `Color::from_str()`.
/// If parsing fails, serde returns an error with the TOML location.
pub(crate) fn deserialize_hex_color<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) => Color::from_str(&s).map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

/// Serializes `Option<Color>` back into a hex string for round-trip consistency.
pub(crate) fn serialize_hex_color<S>(color: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match color {
        Some(c) => serializer.serialize_str(&c.to_hex_string()),
        None => serializer.serialize_none(),
    }
}
