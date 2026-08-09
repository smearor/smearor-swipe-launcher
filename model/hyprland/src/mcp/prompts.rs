use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the hyprland service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HyprlandMcpPrompts {
    /// Comprehensive guide for all Hyprland MCP tools, resources, and capabilities.
    HyprlandOverview,
    /// Quick reference card for common Hyprland MCP operations.
    HyprlandQuickReference,
    /// Guide for window management operations via MCP.
    HyprlandWindowGuide,
    /// Guide for workspace management operations via MCP.
    HyprlandWorkspaceGuide,
}

impl AsRef<str> for HyprlandMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::HyprlandOverview => "hyprland_overview",
            Self::HyprlandQuickReference => "hyprland_quick_reference",
            Self::HyprlandWindowGuide => "hyprland_window_guide",
            Self::HyprlandWorkspaceGuide => "hyprland_workspace_guide",
        }
    }
}

impl FromStr for HyprlandMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "hyprland_overview" => Ok(Self::HyprlandOverview),
            "hyprland_quick_reference" => Ok(Self::HyprlandQuickReference),
            "hyprland_window_guide" => Ok(Self::HyprlandWindowGuide),
            "hyprland_workspace_guide" => Ok(Self::HyprlandWorkspaceGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for HyprlandMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_prompt_names_roundtrip() {
        let all_prompts = [
            HyprlandMcpPrompts::HyprlandOverview,
            HyprlandMcpPrompts::HyprlandQuickReference,
            HyprlandMcpPrompts::HyprlandWindowGuide,
            HyprlandMcpPrompts::HyprlandWorkspaceGuide,
        ];

        for prompt in &all_prompts {
            let name = prompt.as_ref();
            let parsed = HyprlandMcpPrompts::from_str(name).unwrap_or_else(|_| panic!("failed to parse prompt name: {name}"));
            assert_eq!(*prompt, parsed, "prompt roundtrip mismatch for {name}");
        }

        assert_eq!(all_prompts.len(), 4, "expected 4 prompt variants");
    }

    #[test]
    fn unknown_prompt_name_returns_error() {
        assert!(HyprlandMcpPrompts::from_str("hyprland_nonexistent").is_err());
        assert!(HyprlandMcpPrompts::from_str("").is_err());
    }

    #[test]
    fn display_matches_as_ref() {
        let prompt = HyprlandMcpPrompts::HyprlandOverview;
        assert_eq!(format!("{prompt}"), prompt.as_ref());
    }
}
