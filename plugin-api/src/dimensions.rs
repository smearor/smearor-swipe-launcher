use serde::Deserialize;
use typed_builder::TypedBuilder;

/// Default widget width in pixels for GTK layout.
pub const DEFAULT_WIDGET_WIDTH: i32 = 100;

/// Default widget height in pixels for GTK layout.
pub const DEFAULT_WIDGET_HEIGHT: i32 = 100;

/// Widget dimensions for GTK layout hints.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// fields `width` and `height` map directly to this struct. Both fields are
/// optional — when `None`, the widget uses `DEFAULT_WIDGET_WIDTH` or
/// `DEFAULT_WIDGET_HEIGHT` respectively.
///
/// These dimensions are GTK layout hints (`width_request` / `height_request`)
/// and do not affect graphic (headless) rendering, where the physical button
/// dimensions are provided by the device.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetDimensions {
    /// The width of the widget in pixels.
    #[builder(default, setter(into))]
    pub width: Option<i32>,

    /// The height of the widget in pixels.
    #[builder(default, setter(into))]
    pub height: Option<i32>,
}

impl Default for WidgetDimensions {
    fn default() -> Self {
        Self {
            width: Some(DEFAULT_WIDGET_WIDTH),
            height: Some(DEFAULT_WIDGET_HEIGHT),
        }
    }
}

impl WidgetDimensions {
    /// Returns the width, falling back to `DEFAULT_WIDGET_WIDTH` if `None`.
    pub fn width_or_default(&self) -> i32 {
        self.width.unwrap_or(DEFAULT_WIDGET_WIDTH)
    }

    /// Returns the height, falling back to `DEFAULT_WIDGET_HEIGHT` if `None`.
    pub fn height_or_default(&self) -> i32 {
        self.height.unwrap_or(DEFAULT_WIDGET_HEIGHT)
    }
}
