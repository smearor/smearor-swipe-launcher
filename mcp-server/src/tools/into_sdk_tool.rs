use rust_mcp_sdk::schema::Tool;
use rust_mcp_sdk::schema::ToolAnnotations as SdkToolAnnotations;

use crate::tools::ToolResolver;

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
    /// Optional human-readable title for UI display.
    fn title(&self) -> Option<&str> {
        None
    }
    /// Optional behavioral hints for the tool.
    fn annotations(&self) -> Option<&smearor_model_mcp::ToolAnnotations> {
        None
    }
}

impl<T: SdkToolFields> IntoSdkTool for T {
    fn into_sdk_tool(&self) -> Tool {
        let annotations = self.annotations().map(|a| SdkToolAnnotations {
            title: a.title.clone(),
            read_only_hint: a.read_only_hint,
            destructive_hint: a.destructive_hint,
            idempotent_hint: a.idempotent_hint,
            open_world_hint: a.open_world_hint,
        });
        Tool {
            name: self.name().to_string(),
            description: Some(self.description().to_string()),
            input_schema: ToolResolver::json_schema_to_tool_input_schema(self.input_schema()),
            annotations,
            execution: None,
            icons: vec![],
            meta: None,
            output_schema: None,
            title: self.title().map(|t| t.to_string()),
        }
    }
}
