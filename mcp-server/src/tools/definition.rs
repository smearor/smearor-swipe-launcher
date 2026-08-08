use crate::CloseAreaParams;
use crate::FocusAreaParams;
use crate::GetAreaConfigParams;
use crate::ListAllAreasParams;
use crate::ListAreasParams;
use crate::ListInstancesParams;
use crate::LoadInstanceParams;
use crate::OpenAreaParams;
use crate::OpenTransientAreaParams;
use crate::ReloadInstanceParams;
use crate::SendMessageParams;
use crate::SendMultipleMessagesParams;
use crate::StartInstanceParams;
use crate::StopInstanceParams;
use crate::ToggleAreaParams;
use crate::UnloadInstanceParams;
use crate::WebServerStatusParams;
use crate::tools::creator::ToolDefinitionCreator;
use crate::tools::handler::ToolHandler;
use crate::tools::into_sdk_tool::SdkToolFields;
use serde_json::Value;

/// Built-in tool definitions exposed by the MCP server.
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub handler: ToolHandler,
}

impl ToolDefinition {
    /// Build the list of core tools available from the MVP.
    pub fn core_tools() -> Vec<ToolDefinition> {
        vec![
            OpenAreaParams::create_tool_definition(),
            CloseAreaParams::create_tool_definition(),
            ListAreasParams::create_tool_definition(),
            OpenTransientAreaParams::create_tool_definition(),
            FocusAreaParams::create_tool_definition(),
            SendMessageParams::create_tool_definition(),
            SendMultipleMessagesParams::create_tool_definition(),
            ToggleAreaParams::create_tool_definition(),
            ListAllAreasParams::create_tool_definition(),
            GetAreaConfigParams::create_tool_definition(),
            LoadInstanceParams::create_tool_definition(),
            StartInstanceParams::create_tool_definition(),
            StopInstanceParams::create_tool_definition(),
            UnloadInstanceParams::create_tool_definition(),
            ReloadInstanceParams::create_tool_definition(),
            ListInstancesParams::create_tool_definition(),
            WebServerStatusParams::create_tool_definition(),
        ]
    }
}

impl SdkToolFields for ToolDefinition {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn input_schema(&self) -> &serde_json::Value {
        &self.input_schema
    }
}
