use rust_mcp_sdk::schema::Tool;
use rust_mcp_sdk::schema::ToolInputSchema;

use crate::tools::json_schema_to_tool_input_schema;

/// Trait for converting tool-like types into the SDK `Tool` type.
pub trait IntoSdkTool {
    /// Convert into the SDK `Tool` representation.
    fn into_sdk_tool(&self) -> Tool;
}

/// Fields shared by all tool-like types that can be converted to an SDK `Tool`.
pub trait SdkToolFields {
    /// The MCP tool name (e.g. "open_area").
    fn name(&self) -> &str;
    /// The human-readable description shown to the LLM.
    fn description(&self) -> &str;
    /// The JSON schema describing the tool's input parameters.
    fn input_schema(&self) -> &serde_json::Value;
}

impl<T: SdkToolFields> IntoSdkTool for T {
    fn into_sdk_tool(&self) -> Tool {
        Tool {
            name: self.name().to_string(),
            description: Some(self.description().to_string()),
            input_schema: json_schema_to_tool_input_schema(self.input_schema()),
            annotations: None,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: None,
        }
    }
}
