use crate::service::VoiceAssistantService;
use smearor_voice_assistant_model::ToolCatalogEntry;
use tracing::debug;

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
        self.invalidate_all_tool_cache();
        self.rebuild_tool_router();
    }

    /// Invalidates all cached results for a specific tool.
    /// Should be called when a tool's behavior changes or it is unregistered.
    #[allow(dead_code)]
    pub fn invalidate_tool_cache(&self, tool_name: &str) {
        self.tool_cache.invalidate_tool(tool_name);
    }

    /// Invalidates the entire tool result cache.
    /// Should be called when the tool set changes significantly.
    pub fn invalidate_all_tool_cache(&self) {
        self.tool_cache.invalidate_all();
    }

    /// Builds the stable system prompt (no dynamic content).
    /// This prompt is cached in the KV-cache and reused across commands.
    /// Tools, entity states, and long-term facts are NOT included here —
    /// they go into `build_context_message` instead.
    pub fn build_system_prompt(&self) -> String {
        const DEFAULT_PROMPT: &str = "You are a desktop assistant for the Smearor Swipe Launcher. \
            You control the system via tool calls. \
            The available tools and current context are provided in the first user message. \
            Respond ONLY in JSON format — no other text. \
            To call a tool, output: {\"tool\": \"<name>\", \"arguments\": {...}}. \
            To give a final answer, output: {\"final_answer\": \"<text>\"}. \
            After a tool has been executed successfully, always respond with a final_answer. \
            Never call the same tool twice in a row. \
            If no available tool matches the user's request, respond with a final_answer explaining that you cannot help with this request. \
            Never use system_power_action, system_reboot_to_uefi, or system_schedule_power_action unless the user explicitly asks to shutdown, reboot, or schedule a power action. \
            Be concise and efficient. Prefer single tool calls when possible.";

        self.config.system_prompt.as_deref().unwrap_or(DEFAULT_PROMPT).to_string()
    }

    /// Builds the dynamic context message injected as the first user message.
    /// This changes per command but does NOT trigger a worker reset.
    /// Contains: nucleo-selected tools, entity states, recalled long-term facts.
    pub fn build_context_message(&self, user_text: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        let selected_tools = self.select_tools_for_prompt(user_text);
        let tools_json = self.serialize_tools(&selected_tools);
        parts.push(format!("Available tools: {tools_json}"));

        if self.config.inject_entity_states {
            let entity_summary = self.build_entity_summary();
            if !entity_summary.is_empty() {
                parts.push(format!("Known device states:\n{entity_summary}"));
            }
        }

        if self.config.inject_long_term_facts {
            let long_term_summary = self.build_long_term_summary(user_text);
            if !long_term_summary.is_empty() {
                parts.push(format!("Known facts:\n{long_term_summary}"));
            }
        }

        parts.join("\n")
    }

    /// Selects tools for the context message using the nucleo tool router.
    fn select_tools_for_prompt(&self, user_text: &str) -> Vec<ToolCatalogEntry> {
        let start = std::time::Instant::now();
        let result = if let Ok(router) = self.tool_router.read() {
            router.select_tools(user_text, self.config.max_tools_in_prompt)
        } else {
            Vec::new()
        };
        self.performance_monitor.record_tool_selection(start.elapsed());
        result
    }

    /// Serializes tool entries to a compact JSON string.
    /// Results are cached with moka keyed by tool names to avoid repeated serialization.
    fn serialize_tools(&self, tools: &[ToolCatalogEntry]) -> String {
        let cache_key: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
        if let Some(cached) = self.tools_json_cache.get(&cache_key) {
            return cached;
        }

        let tools_json: Vec<serde_json::Value> = tools
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
        let result = serde_json::to_string(&tools_json).unwrap_or_default();
        self.tools_json_cache.insert(cache_key, result.clone());
        result
    }

    /// Builds a summary of current entity states for context injection.
    fn build_entity_summary(&self) -> String {
        let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
        if store.is_empty() {
            return String::new();
        }
        let mut lines: Vec<String> = store.values().map(|state| format!("- {}: {}", state.name, state.state)).collect();
        lines.sort();
        lines.join("\n")
    }

    /// Builds a summary of semantically recalled long-term facts.
    fn build_long_term_summary(&self, user_text: &str) -> String {
        let mut memory = match self.semantic_memory.write() {
            Ok(memory) => memory,
            Err(_) => return String::new(),
        };
        let start = std::time::Instant::now();
        let result = memory.recall(user_text, self.config.max_recalled_facts);
        self.performance_monitor.record_embedding(start.elapsed());
        match result {
            Ok(facts) => {
                if facts.is_empty() {
                    return String::new();
                }
                facts
                    .iter()
                    .map(|fact| format!("- {} ({}): {}", fact.key, fact.category, fact.value))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Err(error) => {
                debug!("Voice Assistant: long-term recall failed: {error}");
                String::new()
            }
        }
    }
}
