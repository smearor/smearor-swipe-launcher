use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandSwitchXkbLayoutCmdKind;

/// The kind of switch-xkb-layout command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpSwitchXkbLayoutCmdKind {
    /// Switch to the next layout.
    #[default]
    Next,
    /// Switch to the previous layout.
    Previous,
    /// Switch to a specific layout by id.
    Id,
}

impl From<McpSwitchXkbLayoutCmdKind> for HyprlandSwitchXkbLayoutCmdKind {
    fn from(value: McpSwitchXkbLayoutCmdKind) -> Self {
        match value {
            McpSwitchXkbLayoutCmdKind::Next => HyprlandSwitchXkbLayoutCmdKind::Next,
            McpSwitchXkbLayoutCmdKind::Previous => HyprlandSwitchXkbLayoutCmdKind::Previous,
            McpSwitchXkbLayoutCmdKind::Id => HyprlandSwitchXkbLayoutCmdKind::Id,
        }
    }
}
