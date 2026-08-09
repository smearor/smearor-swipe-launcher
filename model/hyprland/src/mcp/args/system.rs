use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::types::McpLockType;
use super::types::McpWindowIdentifier;

/// Arguments for the `hyprland_system_add_master` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct AddMasterArgs {}

/// Arguments for the `hyprland_system_bring_active_to_top` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct BringActiveToTopArgs {}

/// Arguments for the `hyprland_system_custom` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct CustomDispatchArgs {
    /// The custom dispatch name
    pub name: String,
    /// The custom dispatch value
    pub value: String,
}

/// Arguments for the `hyprland_system_exit` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ExitArgs {}

/// Arguments for the `hyprland_system_force_renderer_reload` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ForceRendererReloadArgs {}

/// Arguments for the `hyprland_system_global` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct GlobalDispatchArgs {
    /// The global keybinding key to execute
    pub key: String,
}

/// Arguments for the `hyprland_system_lock_groups` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct LockGroupsArgs {
    /// Lock type: lock, unlock, or toggle
    pub lock_type: McpLockType,
}

/// Arguments for the `hyprland_system_move_out_of_group` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveOutOfGroupArgs {}

/// Arguments for the `hyprland_system_orientation` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct OrientationArgs {
    /// The orientation to set: top, bottom, left, right, center, next, or prev
    pub orientation: OrientationKind,
}

/// Orientation kind for the orientation dispatch command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OrientationKind {
    #[default]
    Bottom,
    Center,
    Left,
    Next,
    Prev,
    Right,
    Top,
}

/// Arguments for the `hyprland_system_pass` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct PassArgs {
    /// Window identifier to pass the key event to
    pub window_identifier: McpWindowIdentifier,
}

/// Arguments for the `hyprland_system_remove_master` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct RemoveMasterArgs {}

/// Arguments for the `hyprland_system_set_cursor` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetCursorArgs {
    /// The cursor theme name
    pub theme: String,
    /// The cursor size in pixels
    pub size: u16,
}
