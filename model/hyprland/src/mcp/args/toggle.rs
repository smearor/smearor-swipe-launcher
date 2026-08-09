use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::types::McpFullscreenType;

/// Arguments for the `hyprland_toggle_fullscreen` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct FullscreenTypeArgs {
    /// Fullscreen type: real, maximize, or no-param
    pub fullscreen_type: McpFullscreenType,
}

/// Arguments for the `hyprland_toggle_dpms` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ToggleDpmsArgs {
    /// Whether DPMS should be on or off
    pub on: bool,
    /// Optional monitor name; if omitted, applies to all monitors
    pub name: Option<String>,
}

/// Arguments for the `hyprland_toggle_fake_fullscreen` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ToggleFakeFullscreenArgs {}

/// Arguments for the `hyprland_toggle_group` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ToggleGroupArgs {}

/// Arguments for the `hyprland_toggle_opaque` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ToggleOpaqueArgs {}

/// Arguments for the `hyprland_toggle_pin` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct TogglePinArgs {}

/// Arguments for the `hyprland_toggle_pseudo` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct TogglePseudoArgs {}

/// Arguments for the `hyprland_toggle_split` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ToggleSplitArgs {}
