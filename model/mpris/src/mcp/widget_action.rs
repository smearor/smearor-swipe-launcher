use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// Widget-level tool actions for the MPRIS widget, invoked via `InvokeToolMessage`.
///
/// These actions control view switching between Compact and Expanded views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MprisWidgetAction {
    /// Expand the widget to the Expanded view.
    Expand,
    /// Collapse the widget to the Compact view.
    Collapse,
    /// Toggle between Compact and Expanded views.
    ToggleView,
}

impl AsRef<str> for MprisWidgetAction {
    fn as_ref(&self) -> &str {
        match self {
            Self::Expand => "expand",
            Self::Collapse => "collapse",
            Self::ToggleView => "toggle_view",
        }
    }
}

impl FromStr for MprisWidgetAction {
    type Err = UnknownToolError;

    fn from_str(action: &str) -> Result<Self, Self::Err> {
        match action {
            "expand" => Ok(Self::Expand),
            "collapse" => Ok(Self::Collapse),
            "toggle_view" => Ok(Self::ToggleView),
            _ => Err(UnknownToolError::new(action)),
        }
    }
}

impl Display for MprisWidgetAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
