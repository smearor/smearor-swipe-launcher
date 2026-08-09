use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandFullscreenType;

/// Fullscreen mode for toggle fullscreen commands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpFullscreenType {
    /// Real fullscreen mode, covering the entire screen.
    Real,
    /// Maximized fullscreen mode, filling the workspace.
    Maximize,
    /// No parameter, uses the default fullscreen behavior.
    #[default]
    NoParam,
}

impl From<McpFullscreenType> for HyprlandFullscreenType {
    fn from(value: McpFullscreenType) -> Self {
        match value {
            McpFullscreenType::Real => HyprlandFullscreenType::Real,
            McpFullscreenType::Maximize => HyprlandFullscreenType::Maximize,
            McpFullscreenType::NoParam => HyprlandFullscreenType::NoParam,
        }
    }
}
