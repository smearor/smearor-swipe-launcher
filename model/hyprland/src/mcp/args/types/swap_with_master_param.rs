use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandSwapWithMasterParam;

/// Parameter for the swap-with-master dispatch command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpSwapWithMasterParam {
    /// Swap with the master window.
    #[default]
    Master,
    /// Swap with the first child window.
    Child,
    /// Automatically determine which window to swap with.
    Auto,
}

impl From<McpSwapWithMasterParam> for HyprlandSwapWithMasterParam {
    fn from(value: McpSwapWithMasterParam) -> Self {
        match value {
            McpSwapWithMasterParam::Master => HyprlandSwapWithMasterParam::Master,
            McpSwapWithMasterParam::Child => HyprlandSwapWithMasterParam::Child,
            McpSwapWithMasterParam::Auto => HyprlandSwapWithMasterParam::Auto,
        }
    }
}
