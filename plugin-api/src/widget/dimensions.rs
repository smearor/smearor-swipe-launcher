use serde::Deserialize;
use serde::Serialize;
use typed_builder::TypedBuilder;

use crate::widget::WidgetMode;

/// Default widget width in pixels for GTK layout.
pub const DEFAULT_WIDGET_WIDTH: i32 = 100;

/// Default widget height in pixels for GTK layout.
pub const DEFAULT_WIDGET_HEIGHT: i32 = 100;

/// Default maximum widget width in pixels for Wide mode.
pub const DEFAULT_WIDE_MODE_WIDGET_WIDTH: i32 = 300;

/// Widget dimensions for GTK layout hints.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// fields `width`, `height`, and `max_width` map directly to this struct.
/// All fields are optional — when `None`, the widget uses the corresponding
/// default constant.
///
/// These dimensions are GTK layout hints (`width_request` / `height_request`)
/// and do not affect graphic (headless) rendering, where the physical button
/// dimensions are provided by the device.
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetDimensions {
    /// The width of the widget in pixels.
    #[builder(default, setter(into))]
    pub width: Option<i32>,

    /// The height of the widget in pixels.
    #[builder(default, setter(into))]
    pub height: Option<i32>,

    /// The maximum width of the widget in pixels.
    #[builder(default, setter(into))]
    pub max_width: Option<i32>,
}

impl Default for WidgetDimensions {
    fn default() -> Self {
        Self {
            width: Some(DEFAULT_WIDGET_WIDTH),
            height: Some(DEFAULT_WIDGET_HEIGHT),
            max_width: None,
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

    /// Returns the maximum width, falling back to a mode-dependent default if `None`.
    ///
    /// In Wide mode, the default is `DEFAULT_WIDE_MODE_WIDGET_WIDTH` (300px).
    /// In Compact mode (or when mode is not applicable), the default is
    /// `DEFAULT_WIDGET_WIDTH` (100px).
    pub fn max_width_or_default(&self, mode: WidgetMode) -> i32 {
        self.max_width.unwrap_or(match mode {
            WidgetMode::Wide => DEFAULT_WIDE_MODE_WIDGET_WIDTH,
            WidgetMode::Compact => DEFAULT_WIDGET_WIDTH,
        })
    }
}
