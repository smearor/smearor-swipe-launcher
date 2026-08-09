use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandNotifyIcon;

/// Icon type for the notify command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpNotifyIcon {
    /// Warning icon.
    #[default]
    Warning,
    /// Information icon.
    Info,
    /// Hint icon.
    Hint,
    /// Error icon.
    Error,
    /// Confused icon.
    Confused,
    /// OK icon.
    Ok,
    /// No icon displayed.
    NoIcon,
}

impl From<McpNotifyIcon> for HyprlandNotifyIcon {
    fn from(value: McpNotifyIcon) -> Self {
        match value {
            McpNotifyIcon::Warning => HyprlandNotifyIcon::Warning,
            McpNotifyIcon::Info => HyprlandNotifyIcon::Info,
            McpNotifyIcon::Hint => HyprlandNotifyIcon::Hint,
            McpNotifyIcon::Error => HyprlandNotifyIcon::Error,
            McpNotifyIcon::Confused => HyprlandNotifyIcon::Confused,
            McpNotifyIcon::Ok => HyprlandNotifyIcon::Ok,
            McpNotifyIcon::NoIcon => HyprlandNotifyIcon::NoIcon,
        }
    }
}
