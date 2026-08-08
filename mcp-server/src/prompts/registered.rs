use smearor_model_mcp::RegisteredPrompt;

use crate::prompts::into_sdk_prompt::SdkPromptFields;

impl SdkPromptFields for RegisteredPrompt {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn arguments_schema(&self) -> &serde_json::Value {
        &self.arguments_schema
    }
}
