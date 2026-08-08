use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the hyprland service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HyprlandMcpTools {
    /// Switch to a workspace by ID.
    SwitchWorkspace,
    /// Move the active window to a workspace.
    MoveWindowToWorkspace,
    /// Toggle floating mode for the active window.
    ToggleFloating,
}

impl AsRef<str> for HyprlandMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::SwitchWorkspace => "hyprland_switch_workspace",
            Self::MoveWindowToWorkspace => "hyprland_move_window",
            Self::ToggleFloating => "hyprland_toggle_floating",
        }
    }
}

impl FromStr for HyprlandMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "hyprland_switch_workspace" => Ok(Self::SwitchWorkspace),
            "hyprland_move_window" => Ok(Self::MoveWindowToWorkspace),
            "hyprland_toggle_floating" => Ok(Self::ToggleFloating),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for HyprlandMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
