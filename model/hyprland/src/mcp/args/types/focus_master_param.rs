use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandFocusMasterParam;

/// Parameter for the focus-master dispatch command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpFocusMasterParam {
    /// Focus the master window.
    #[default]
    Master,
    /// Automatically determine which window to focus.
    Auto,
}

impl From<McpFocusMasterParam> for HyprlandFocusMasterParam {
    fn from(value: McpFocusMasterParam) -> Self {
        match value {
            McpFocusMasterParam::Master => HyprlandFocusMasterParam::Master,
            McpFocusMasterParam::Auto => HyprlandFocusMasterParam::Auto,
        }
    }
}
