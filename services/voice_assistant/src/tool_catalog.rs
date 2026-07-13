use smearor_voice_assistant_model::ToolCatalogEntry;
use tracing::debug;

use crate::service::VoiceAssistantService;

impl VoiceAssistantService {
    /// Handles a new tool registration from the broker.
    pub fn on_tool_registered(&self, name: String, description: String, input_schema: String) {
        let entry = ToolCatalogEntry {
            name,
            description,
            input_schema,
        };
        if let Ok(mut catalog) = self.tool_catalog.write() {
            catalog.push(entry);
        }
    }

    /// Builds the system prompt for the LLM, injecting the tool catalog.
    ///
    /// If the serialized tool catalog exceeds the token budget (default: ~1024 tokens / ~4000 chars),
    /// tools are truncated to fit. Each tool entry includes name, description, and a compressed
    /// input schema (required fields only). This prevents the system prompt from consuming
    /// the majority of the context window.
    pub fn build_system_prompt(&self) -> String {
        let catalog = self.tool_catalog.read().unwrap_or_else(|e| e.into_inner());
        let catalog = catalog.iter().collect::<Vec<_>>();
        let max_catalog_chars = self.config.max_catalog_chars;

        let mut tools_json: Vec<serde_json::Value> = catalog
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": serde_json::from_str::<serde_json::Value>(&t.input_schema)
                        .unwrap_or(serde_json::Value::Null),
                })
            })
            .collect();

        let mut serialized = serde_json::to_string(&tools_json).unwrap_or_default();
        while serialized.len() > max_catalog_chars && !tools_json.is_empty() {
            tools_json.pop();
            serialized = serde_json::to_string(&tools_json).unwrap_or_default();
        }

        if tools_json.len() < catalog.len() {
            debug!(
                "Tool catalog truncated: {}/{} tools fit in {} char budget",
                tools_json.len(),
                catalog.len(),
                max_catalog_chars
            );
        }

        const DEFAULT_PROMPT: &str = "You are a desktop assistant for the Smearor Swipe Launcher. \
            You control the system via tool calls. \
            Available tools: {tools}. \
            Respond in JSON format. \
            To call a tool, output: {{\"tool\": \"<name>\", \"arguments\": {{...}}}}. \
            To give a final answer, output: {{\"final_answer\": \"<text>\"}}. \
            Be concise and efficient. Prefer single tool calls when possible.";

        let template = self.config.system_prompt.as_deref().unwrap_or(DEFAULT_PROMPT);
        template.replace("{tools}", &serialized)
    }
}
