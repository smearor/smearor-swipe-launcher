use serde::Deserialize;
use serde::Serialize;
use typed_builder::TypedBuilder;

/// Default spacing between child widgets inside a GTK container.
pub const DEFAULT_WIDGET_SPACING: i32 = 0;

/// Widget layout options for GTK container spacing.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// field `spacing` maps directly to this struct. The field is optional —
/// when `None`, the widget uses `DEFAULT_WIDGET_SPACING`.
///
/// This controls the pixel distance between sibling widgets inside a
/// `GtkBox` container (`GtkBox::spacing`).
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetLayout {
    /// Spacing between child widgets in pixels.
    #[builder(default, setter(into))]
    pub spacing: Option<i32>,
}

impl Default for WidgetLayout {
    fn default() -> Self {
        Self {
            spacing: Some(DEFAULT_WIDGET_SPACING),
        }
    }
}

impl WidgetLayout {
    /// Returns the spacing, falling back to `DEFAULT_WIDGET_SPACING` if `None`.
    pub fn spacing_or_default(&self) -> i32 {
        self.spacing.unwrap_or(DEFAULT_WIDGET_SPACING)
    }
}
