use serde::Deserialize;
use serde::Serialize;

/// Default widget mode.
pub const DEFAULT_WIDGET_MODE: WidgetMode = WidgetMode::Compact;

/// Layout mode for widgets that support both compact and wide presentations.
///
/// In **Compact** mode, the widget stacks its elements vertically (icon, main text,
/// info text) — matching the layout of button and weather widgets.
/// In **Wide** mode, the widget uses a horizontal layout with extended info panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WidgetMode {
    /// Vertical layout: icon on top, main text and info text below.
    Compact,
    /// Horizontal layout: icon on the left, info panels on the right.
    Wide,
}

impl Default for WidgetMode {
    fn default() -> Self {
        DEFAULT_WIDGET_MODE
    }
}
