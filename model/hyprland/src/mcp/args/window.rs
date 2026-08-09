use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::types::McpCorner;
use super::types::McpCycleDirection;
use super::types::McpDirection;
use super::types::McpFocusMasterParam;
use super::types::McpMonitorIdentifier;
use super::types::McpPosition;
use super::types::McpSwapWithMasterParam;
use super::types::McpWindowIdentifier;
use super::types::McpWindowMove;
use super::types::McpWindowSwitchDirection;

/// Arguments for the `hyprland_window_center` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct CenterWindowArgs {}

/// Arguments for the `hyprland_window_change_group_active` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ChangeGroupActiveArgs {
    /// Direction to switch active window in group (back or forward)
    pub direction: McpWindowSwitchDirection,
}

/// Arguments for the `hyprland_window_change_split_ratio` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ChangeSplitRatioArgs {
    /// The new split ratio (0.0 to 1.0)
    pub ratio: f32,
}

/// Arguments for the `hyprland_window_close` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct CloseWindowArgs {
    /// Window identifier (address, class, title, or process ID)
    pub window_identifier: McpWindowIdentifier,
}

/// Arguments for the `hyprland_window_cycle` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct CycleWindowArgs {
    /// Direction to cycle (next or previous)
    pub cycle_direction: McpCycleDirection,
}

/// Arguments for the `hyprland_window_exec` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ExecArgs {
    /// The command to execute
    pub command: String,
}

/// Arguments for the `hyprland_window_focus_current_or_last` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct FocusCurrentOrLastArgs {}

/// Arguments for the `hyprland_window_focus_master` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct FocusMasterArgs {
    /// Whether to focus master or auto-select
    pub param: McpFocusMasterParam,
}

/// Arguments for the `hyprland_window_focus_monitor` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct FocusMonitorArgs {
    /// Monitor identifier (current, direction, id, name, or relative)
    pub monitor_identifier: McpMonitorIdentifier,
}

/// Arguments for the `hyprland_window_focus_urgent_or_last` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct FocusUrgentOrLastArgs {}

/// Arguments for the `hyprland_window_focus_window` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct FocusWindowArgs {
    /// Window identifier (address, class, title, or process ID)
    pub window_identifier: McpWindowIdentifier,
}

/// Arguments for the `hyprland_window_kill_active` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct KillActiveWindowArgs {}

/// Arguments for the `hyprland_window_move_active` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveActiveArgs {
    /// Position delta or exact coordinates
    pub position: McpPosition,
}

/// Arguments for the `hyprland_window_move_cursor` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveCursorArgs {
    /// X coordinate
    pub x: i64,
    /// Y coordinate
    pub y: i64,
}

/// Arguments for the `hyprland_window_move_cursor_to_corner` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveCursorToCornerArgs {
    /// Which corner to move to
    pub corner: McpCorner,
}

/// Arguments for the `hyprland_window_move_focus` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveFocusArgs {
    /// Direction to move focus (up, down, left, right)
    pub direction: McpDirection,
}

/// Arguments for the `hyprland_window_move_into_group` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveIntoGroupArgs {
    /// Direction to move into group (up, down, left, right)
    pub direction: McpDirection,
}

/// Arguments for the `hyprland_window_move_window` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveWindowDispatchArgs {
    /// Window move parameters (direction or monitor)
    pub window_move: McpWindowMove,
}

/// Arguments for the `hyprland_window_move_window_pixel` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MoveWindowPixelArgs {
    /// Position delta or exact coordinates
    pub position: McpPosition,
    /// Window identifier (address, class, title, or process ID)
    pub window_identifier: McpWindowIdentifier,
}

/// Arguments for the `hyprland_window_resize_active` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ResizeActiveArgs {
    /// Position delta or exact coordinates for resize
    pub position: McpPosition,
}

/// Arguments for the `hyprland_window_resize_window_pixel` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ResizeWindowPixelArgs {
    /// Position delta or exact coordinates for resize
    pub position: McpPosition,
    /// Window identifier (address, class, title, or process ID)
    pub window_identifier: McpWindowIdentifier,
}

/// Arguments for the `hyprland_window_swap` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SwapWindowArgs {
    /// Direction to swap (next or previous)
    pub cycle_direction: McpCycleDirection,
}

/// Arguments for the `hyprland_window_swap_with_master` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SwapWithMasterArgs {
    /// Whether to swap with master, child, or auto
    pub param: McpSwapWithMasterParam,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_args_struct_deserializes_from_empty_object() {
        let _args: CenterWindowArgs = serde_json::from_str("{}").unwrap();
    }

    #[test]
    fn no_args_struct_deserializes_from_empty_string() {
        let _args: CenterWindowArgs = serde_json::from_str("").unwrap_or_default();
    }

    #[test]
    fn exec_args_roundtrip() {
        let original = ExecArgs { command: "kitty".to_string() };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ExecArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, original.command);
    }

    #[test]
    fn move_cursor_args_roundtrip() {
        let original = MoveCursorArgs { x: 100, y: 200 };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: MoveCursorArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.x, 100);
        assert_eq!(parsed.y, 200);
    }

    #[test]
    fn change_split_ratio_args_roundtrip() {
        let original = ChangeSplitRatioArgs { ratio: 0.5 };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ChangeSplitRatioArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ratio, 0.5);
    }

    #[test]
    fn invalid_json_falls_back_to_default() {
        let args: MoveCursorArgs = serde_json::from_str("not json").unwrap_or_default();
        assert_eq!(args.x, 0);
        assert_eq!(args.y, 0);
    }

    #[test]
    fn missing_field_uses_default() {
        let args: MoveCursorArgs = serde_json::from_str("{}").unwrap_or_default();
        assert_eq!(args.x, 0);
        assert_eq!(args.y, 0);
    }
}
