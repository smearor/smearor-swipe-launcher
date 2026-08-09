use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources registered by the hyprland service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HyprlandMcpResources {
    /// Current Hyprland state (active window, fullscreen, keyboard layout).
    State,
    /// Active window information.
    ActiveWindow,
    /// Workspace snapshot (all workspaces and their states).
    WorkspaceSnapshot,
    /// List of all workspaces.
    Workspaces,
    /// List of all monitors.
    Monitors,
    /// Recent window status events.
    WindowStatus,
    /// Recent workspace status events.
    WorkspaceStatus,
    /// Recent group status events.
    GroupStatus,
    /// Recent layer shell status events.
    LayerStatus,
    /// Recent system status events.
    SystemStatus,
}

impl AsRef<str> for HyprlandMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::State => "hyprland://state",
            Self::ActiveWindow => "hyprland://active-window",
            Self::WorkspaceSnapshot => "hyprland://workspace-snapshot",
            Self::Workspaces => "hyprland://workspaces",
            Self::Monitors => "hyprland://monitors",
            Self::WindowStatus => "hyprland://window-status",
            Self::WorkspaceStatus => "hyprland://workspace-status",
            Self::GroupStatus => "hyprland://group-status",
            Self::LayerStatus => "hyprland://layer-status",
            Self::SystemStatus => "hyprland://system-status",
        }
    }
}

impl FromStr for HyprlandMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "hyprland://state" => Ok(Self::State),
            "hyprland://active-window" => Ok(Self::ActiveWindow),
            "hyprland://workspace-snapshot" => Ok(Self::WorkspaceSnapshot),
            "hyprland://workspaces" => Ok(Self::Workspaces),
            "hyprland://monitors" => Ok(Self::Monitors),
            "hyprland://window-status" => Ok(Self::WindowStatus),
            "hyprland://workspace-status" => Ok(Self::WorkspaceStatus),
            "hyprland://group-status" => Ok(Self::GroupStatus),
            "hyprland://layer-status" => Ok(Self::LayerStatus),
            "hyprland://system-status" => Ok(Self::SystemStatus),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for HyprlandMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_resource_uris_roundtrip() {
        let all_resources = [
            HyprlandMcpResources::State,
            HyprlandMcpResources::ActiveWindow,
            HyprlandMcpResources::WorkspaceSnapshot,
            HyprlandMcpResources::Workspaces,
            HyprlandMcpResources::Monitors,
            HyprlandMcpResources::WindowStatus,
            HyprlandMcpResources::WorkspaceStatus,
            HyprlandMcpResources::GroupStatus,
            HyprlandMcpResources::LayerStatus,
            HyprlandMcpResources::SystemStatus,
        ];

        for resource in &all_resources {
            let uri = resource.as_ref();
            let parsed = HyprlandMcpResources::from_str(uri).unwrap_or_else(|_| panic!("failed to parse resource URI: {uri}"));
            assert_eq!(*resource, parsed, "resource roundtrip mismatch for {uri}");
        }

        assert_eq!(all_resources.len(), 10, "expected 10 resource variants");
    }

    #[test]
    fn unknown_resource_uri_returns_error() {
        assert!(HyprlandMcpResources::from_str("hyprland://nonexistent").is_err());
        assert!(HyprlandMcpResources::from_str("").is_err());
        assert!(HyprlandMcpResources::from_str("hyprland://").is_err());
    }
}
