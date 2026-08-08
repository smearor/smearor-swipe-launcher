use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::ActionKind;

/// Arguments for tools with no parameters.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct NoArgs {}

/// Arguments for widget button MCP tools.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ButtonActionArgs {
    /// The button action to trigger.
    pub action: ActionKind,
}
