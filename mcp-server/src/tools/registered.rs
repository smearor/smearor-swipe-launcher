use smearor_model_mcp::RegisteredTool;

use crate::tools::into_sdk_tool::SdkToolFields;

impl SdkToolFields for RegisteredTool {
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
