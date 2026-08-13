use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `set_theme` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SetThemeArgs {
    /// Name of the theme to select and apply.
    pub name: String,
}
