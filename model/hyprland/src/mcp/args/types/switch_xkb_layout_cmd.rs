use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandSwitchXkbLayoutCmd;

use crate::mcp::args::types::switch_xkb_layout_cmd_kind::McpSwitchXkbLayoutCmdKind;

/// Parameters for switching the XKB keyboard layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct McpSwitchXkbLayoutCmd {
    /// The kind of command.
    pub kind: McpSwitchXkbLayoutCmdKind,
    /// Layout id for the Id variant.
    pub id: u8,
}

impl From<McpSwitchXkbLayoutCmd> for HyprlandSwitchXkbLayoutCmd {
    fn from(value: McpSwitchXkbLayoutCmd) -> Self {
        HyprlandSwitchXkbLayoutCmd {
            kind: value.kind.into(),
            id: value.id,
        }
    }
}
