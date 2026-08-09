use gtk4::Align;
use gtk4::Button;
use gtk4::Widget;
use gtk4::prelude::IsA;
use serde::Deserialize;
use serde::Serialize;
use typed_builder::TypedBuilder;

use crate::widget::WidgetMode;
use crate::widget::css::register_css_once;

/// Default widget width in pixels for GTK layout.
pub const DEFAULT_WIDGET_WIDTH: i32 = 100;

/// Default widget height in pixels for GTK layout.
pub const DEFAULT_WIDGET_HEIGHT: i32 = 100;

/// Default maximum widget width in pixels for Wide mode.
pub const DEFAULT_WIDE_MODE_WIDGET_WIDTH: i32 = 300;

/// Widget dimensions for GTK layout hints.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// fields `width`, `height`, `max_width`, and `scale` map directly to this struct.
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

    /// Per-widget scaling factor that overrides the global `[launcher]` scale.
    ///
    /// When `None`, the widget uses the global scale injected by the launcher.
    /// When `Some(value)`, this value replaces the global scale for this widget
    /// only — it is NOT multiplied on top of the global scale.
    ///
    /// Affects all pixel-based dimensions: width, height, max_width, icon_size,
    /// label heights (20px/16px), spacing, and CSS font sizes.
    ///
    /// Values are sanitized via `sanitize_scale()` (clamped to [0.5, 3.0],
    /// NaN/infinity → 1.0) before use.
    #[builder(default, setter(into))]
    #[serde(default)]
    pub scale: Option<f32>,
}

impl Default for WidgetDimensions {
    fn default() -> Self {
        Self {
            width: Some(DEFAULT_WIDGET_WIDTH),
            height: Some(DEFAULT_WIDGET_HEIGHT),
            max_width: None,
            scale: None,
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

    /// Returns the effective widget width: `min(width, max_width)`.
    pub fn effective_width(&self, mode: WidgetMode) -> i32 {
        self.width_or_default().min(self.max_width_or_default(mode))
    }

    /// Returns the width, scaled by the given factor.
    pub fn width_scaled(&self, scale: f32) -> i32 {
        ((self.width.unwrap_or(DEFAULT_WIDGET_WIDTH) as f32) * scale).round() as i32
    }

    /// Returns the height, scaled by the given factor.
    pub fn height_scaled(&self, scale: f32) -> i32 {
        ((self.height.unwrap_or(DEFAULT_WIDGET_HEIGHT) as f32) * scale).round() as i32
    }

    /// Returns the max width, scaled by the given factor.
    pub fn max_width_scaled(&self, mode: WidgetMode, scale: f32) -> i32 {
        let default = match mode {
            WidgetMode::Wide => DEFAULT_WIDE_MODE_WIDGET_WIDTH,
            WidgetMode::Compact => DEFAULT_WIDGET_WIDTH,
        };
        ((self.max_width.unwrap_or(default) as f32) * scale).round() as i32
    }

    /// Returns the effective widget width: `min(width, max_width)`, both scaled.
    pub fn effective_width_scaled(&self, mode: WidgetMode, scale: f32) -> i32 {
        self.width_scaled(scale).min(self.max_width_scaled(mode, scale))
    }

    /// Builds a `Button` with scaled dimensions.
    ///
    /// When `max_width` is set, applies a CSS `max-width` constraint using the given
    /// CSS class prefix (e.g. `"max-width-"` or `"app-launcher-max-width-"`).
    pub fn build_button_scaled(&self, mode: WidgetMode, content: &impl IsA<Widget>, max_width_css_prefix: &str, scale: f32) -> Button {
        let builder = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.effective_width_scaled(mode, scale))
            .height_request(self.height_scaled(scale))
            .child(content);

        if let Some(max_w) = self.max_width {
            let scaled_max_w = ((max_w as f32) * scale).round() as i32;
            let css_class = format!("{}{}", max_width_css_prefix, scaled_max_w);
            let builder = builder
                .hexpand(false)
                .halign(Align::Start)
                .css_classes(["scroll-item", "menu-button", css_class.as_str()]);
            let css = format!(".{}{} {{ max-width: {}px; }}", max_width_css_prefix, scaled_max_w, scaled_max_w);
            register_css_once(&css_class, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            builder.build()
        } else {
            builder.build()
        }
    }

    /// Builds a `Button` with standard CSS classes, effective width, height, and content.
    ///
    /// When `max_width` is set, applies a CSS `max-width` constraint using the given
    /// CSS class prefix (e.g. `"max-width-"` or `"app-launcher-max-width-"`).
    pub fn build_button(&self, mode: WidgetMode, content: &impl IsA<Widget>, max_width_css_prefix: &str) -> Button {
        let builder = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.effective_width(mode))
            .height_request(self.height_or_default())
            .child(content);

        if let Some(max_w) = self.max_width {
            let css_class = format!("{}{}", max_width_css_prefix, max_w);
            let builder = builder
                .hexpand(false)
                .halign(Align::Start)
                .css_classes(["scroll-item", "menu-button", css_class.as_str()]);
            let css = format!(".{}{} {{ max-width: {}px; }}", max_width_css_prefix, max_w, max_w);
            register_css_once(&css_class, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            builder.build()
        } else {
            builder.build()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_scaled_uses_default_when_none() {
        let dims = WidgetDimensions {
            width: None,
            height: None,
            max_width: None,
            scale: None,
        };
        assert_eq!(dims.width_scaled(1.0), DEFAULT_WIDGET_WIDTH);
        assert_eq!(dims.width_scaled(2.0), 200);
        assert_eq!(dims.width_scaled(0.5), 50);
    }

    #[test]
    fn width_scaled_uses_configured_value() {
        let dims = WidgetDimensions {
            width: Some(150),
            height: None,
            max_width: None,
            scale: None,
        };
        assert_eq!(dims.width_scaled(1.0), 150);
        assert_eq!(dims.width_scaled(2.0), 300);
        assert_eq!(dims.width_scaled(0.5), 75);
    }

    #[test]
    fn height_scaled_uses_default_when_none() {
        let dims = WidgetDimensions {
            width: None,
            height: None,
            max_width: None,
            scale: None,
        };
        assert_eq!(dims.height_scaled(1.0), DEFAULT_WIDGET_HEIGHT);
        assert_eq!(dims.height_scaled(2.0), 200);
        assert_eq!(dims.height_scaled(0.5), 50);
    }

    #[test]
    fn height_scaled_uses_configured_value() {
        let dims = WidgetDimensions {
            width: None,
            height: Some(80),
            max_width: None,
            scale: None,
        };
        assert_eq!(dims.height_scaled(1.0), 80);
        assert_eq!(dims.height_scaled(1.5), 120);
    }

    #[test]
    fn max_width_scaled_wide_mode_uses_wide_default() {
        let dims = WidgetDimensions {
            width: None,
            height: None,
            max_width: None,
            scale: None,
        };
        assert_eq!(dims.max_width_scaled(WidgetMode::Wide, 1.0), DEFAULT_WIDE_MODE_WIDGET_WIDTH);
        assert_eq!(dims.max_width_scaled(WidgetMode::Wide, 2.0), 600);
    }

    #[test]
    fn max_width_scaled_compact_mode_uses_default() {
        let dims = WidgetDimensions {
            width: None,
            height: None,
            max_width: None,
            scale: None,
        };
        assert_eq!(dims.max_width_scaled(WidgetMode::Compact, 1.0), DEFAULT_WIDGET_WIDTH);
        assert_eq!(dims.max_width_scaled(WidgetMode::Compact, 2.0), 200);
    }

    #[test]
    fn max_width_scaled_uses_configured_value() {
        let dims = WidgetDimensions {
            width: None,
            height: None,
            max_width: Some(250),
            scale: None,
        };
        assert_eq!(dims.max_width_scaled(WidgetMode::Wide, 1.0), 250);
        assert_eq!(dims.max_width_scaled(WidgetMode::Wide, 2.0), 500);
    }

    #[test]
    fn effective_width_scaled_returns_min_of_width_and_max_width() {
        let dims = WidgetDimensions {
            width: Some(100),
            height: None,
            max_width: Some(80),
            scale: None,
        };
        assert_eq!(dims.effective_width_scaled(WidgetMode::Wide, 1.0), 80);
        assert_eq!(dims.effective_width_scaled(WidgetMode::Wide, 2.0), 160);
    }

    #[test]
    fn effective_width_scaled_falls_back_to_defaults() {
        let dims = WidgetDimensions {
            width: None,
            height: None,
            max_width: None,
            scale: None,
        };
        assert_eq!(dims.effective_width_scaled(WidgetMode::Wide, 1.0), DEFAULT_WIDGET_WIDTH);
        assert_eq!(dims.effective_width_scaled(WidgetMode::Compact, 1.0), DEFAULT_WIDGET_WIDTH);
    }

    #[test]
    fn scale_field_defaults_to_none() {
        let dims = WidgetDimensions::default();
        assert!(dims.scale.is_none());
    }

    #[test]
    fn scale_field_round_trips_through_serde() {
        let dims = WidgetDimensions {
            width: None,
            height: None,
            max_width: None,
            scale: Some(1.5),
        };
        let json = serde_json::to_string(&dims).unwrap();
        let deserialized: WidgetDimensions = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.scale, Some(1.5));
    }

    #[test]
    fn scale_field_absent_in_json_defaults_to_none() {
        let json = r#"{"width":100}"#;
        let dims: WidgetDimensions = serde_json::from_str(json).unwrap();
        assert!(dims.scale.is_none());
    }
}
