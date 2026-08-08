use schemars::JsonSchema;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::command::McpCommand;
use crate::command::McpCommandVariant;
use crate::command::wrapper::CommandResponseWrapper;
use crate::tools::ToolDefinitionCreator;

/// Instance type for loading a launcher instance.
#[derive(JsonSchema, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InstanceTypeParam {
    /// Creates a visible window.
    #[default]
    Gtk,
    /// Runs without a window (for hardware devices).
    Headless,
    /// Serves the instance via HTTP.
    Web,
}

impl InstanceTypeParam {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gtk => "gtk",
            Self::Headless => "headless",
            Self::Web => "web",
        }
    }
}

/// Parameters for loading a launcher instance via the command channel.
#[derive(JsonSchema, Deserialize, TypedBuilder)]
pub struct LoadInstanceParams {
    /// Unique identifier for the new instance (e.g. 'side3', 'macropad_5')
    pub instance_id: String,
    /// File system path to the TOML config file (e.g. 'config-side3.toml')
    pub config_path: String,
    /// Instance type: 'gtk' creates a visible window, 'headless' runs without a window, 'web' serves via HTTP
    #[serde(default)]
    #[builder(default)]
    pub instance_type: InstanceTypeParam,
    /// Whether to persist this instance to the state file so it survives restarts.
    #[serde(default)]
    #[builder(default)]
    pub persist: bool,
}

impl McpCommandVariant for LoadInstanceParams {
    fn into_command(wrapper: CommandResponseWrapper<Self>) -> McpCommand {
        McpCommand::LoadInstance(wrapper)
    }
}

impl ToolDefinitionCreator for LoadInstanceParams {
    fn tool_name() -> &'static str {
        "launcher_load_instance"
    }
    fn tool_description() -> &'static str {
        "Dynamically loads a new launcher instance from a TOML config file path. The instance gets its own window, plugins, and areas. Use this to add a new launcher window at runtime."
    }
}
