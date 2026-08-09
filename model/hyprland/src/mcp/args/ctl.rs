use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::types::McpColor;
use super::types::McpNotifyIcon;
use super::types::McpOutputBackend;
use super::types::McpPropType;
use super::types::McpSwitchXkbLayoutCmd;

/// Arguments for the `hyprland_ctl_kill` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct KillArgs {}

/// Arguments for the `hyprland_ctl_notify` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NotifyArgs {
    /// The icon to display with the notification
    pub icon: McpNotifyIcon,
    /// The duration of the notification in milliseconds
    pub time_ms: u32,
    /// The color of the notification
    pub color: McpColor,
    /// The notification message text
    pub message: String,
}

/// Arguments for the `hyprland_ctl_output_create` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct OutputCreateArgs {
    /// The backend to use for the virtual output
    pub backend: McpOutputBackend,
}

/// Arguments for the `hyprland_ctl_output_remove` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct OutputRemoveArgs {
    /// The name of the virtual output to remove
    pub name: String,
}

/// Arguments for the `hyprland_ctl_plugin_load` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct PluginLoadArgs {
    /// The filesystem path to the plugin shared library
    pub path: String,
}

/// Arguments for the `hyprland_ctl_plugin_unload` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct PluginUnloadArgs {
    /// The name of the plugin to unload
    pub name: String,
}

/// Arguments for the `hyprland_ctl_reload` MCP tool (no arguments).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ReloadArgs {}

/// Arguments for the `hyprland_ctl_set_cursor` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetCursorCtlArgs {
    /// The cursor theme name
    pub theme: String,
    /// The cursor size in pixels
    pub size: u16,
}

/// Arguments for the `hyprland_ctl_set_error` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetErrorArgs {
    /// The color of the error message
    pub color: McpColor,
    /// The error message text
    pub message: String,
}

/// Arguments for the `hyprland_ctl_set_prop` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetPropArgs {
    /// The window identifier (e.g. "address:0x1234" or "title:My Window")
    pub identifier: String,
    /// The property to set
    pub prop: McpPropType,
    /// Whether to lock the property
    pub lock: bool,
}

/// Arguments for the `hyprland_ctl_switch_xkb_layout` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SwitchXkbLayoutArgs {
    /// The keyboard device name
    pub device: String,
    /// The layout switch command
    pub cmd: McpSwitchXkbLayoutCmd,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_args_roundtrip() {
        let original = NotifyArgs {
            icon: McpNotifyIcon::default(),
            time_ms: 5000,
            color: McpColor::default(),
            message: "Hello".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: NotifyArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.time_ms, 5000);
        assert_eq!(parsed.message, "Hello");
    }

    #[test]
    fn plugin_load_args_roundtrip() {
        let original = PluginLoadArgs {
            path: "/usr/lib/hyprland/plugins/myplugin.so".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: PluginLoadArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, original.path);
    }

    #[test]
    fn set_cursor_ctl_args_roundtrip() {
        let original = SetCursorCtlArgs {
            theme: "Adwaita".to_string(),
            size: 24,
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: SetCursorCtlArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme, "Adwaita");
        assert_eq!(parsed.size, 24);
    }

    #[test]
    fn set_prop_args_default() {
        let args: SetPropArgs = serde_json::from_str("invalid").unwrap_or_default();
        assert_eq!(args.identifier, "");
        assert_eq!(args.lock, false);
    }

    #[test]
    fn kill_args_from_empty_json() {
        let _args: KillArgs = serde_json::from_str("{}").unwrap();
    }
}
