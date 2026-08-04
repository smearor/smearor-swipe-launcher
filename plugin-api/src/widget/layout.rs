use serde::Deserialize;
use serde::Serialize;
use typed_builder::TypedBuilder;

/// Default spacing between child widgets inside a GTK container.
pub const DEFAULT_WIDGET_SPACING: i32 = 0;

/// Widget layout options for GTK container spacing and CSS classes.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// fields `spacing` and `css_classes` map directly to this struct.
/// `spacing` is optional — when `None`, the widget uses
/// `DEFAULT_WIDGET_SPACING`. `css_classes` defaults to an empty vector.
///
/// `spacing` controls the pixel distance between sibling widgets inside a
/// `GtkBox` container (`GtkBox::spacing`).
/// `css_classes` are user-configurable CSS classes applied to the widget's
/// root GTK widget in addition to the automatic `widget-{plugin_id}` class.
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetLayout {
    /// Spacing between child widgets in pixels.
    #[builder(default, setter(into))]
    pub spacing: Option<i32>,
    /// User-configurable CSS classes applied to the root widget.
    #[builder(default)]
    pub css_classes: Vec<String>,
}

impl Default for WidgetLayout {
    fn default() -> Self {
        Self {
            spacing: Some(DEFAULT_WIDGET_SPACING),
            css_classes: Vec::new(),
        }
    }
}

impl WidgetLayout {
    /// Returns the spacing, falling back to `DEFAULT_WIDGET_SPACING` if `None`.
    pub fn spacing_or_default(&self) -> i32 {
        self.spacing.unwrap_or(DEFAULT_WIDGET_SPACING)
    }
}
