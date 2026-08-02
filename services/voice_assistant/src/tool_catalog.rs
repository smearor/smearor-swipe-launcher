use crate::service::VoiceAssistantService;
use smearor_voice_assistant_model::PromptCatalogEntry;
use smearor_voice_assistant_model::ResourceCatalogEntry;
use smearor_voice_assistant_model::ToolCatalogEntry;
use smearor_voice_assistant_model::xdg_config_path;
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
            catalog.retain(|t| t.name != entry.name);
            catalog.push(entry);
        }
        self.invalidate_all_tool_cache();
        if let Ok(router) = self.tool_router.read() {
            router.mark_dirty();
        }
    }

    /// Handles a new resource registration from the broker.
    pub fn on_resource_registered(&self, uri: String, name: String, description: String, mime_type: String) {
        let entry = ResourceCatalogEntry {
            uri,
            name,
            description,
            mime_type,
        };
        if let Ok(mut catalog) = self.resource_catalog.write() {
            catalog.retain(|r| r.uri != entry.uri);
            catalog.push(entry);
        }
        if let Ok(router) = self.resource_router.read() {
            router.mark_dirty();
        }
        debug!(
            "Voice Assistant: resource catalog updated, {} entries",
            self.resource_catalog.read().map(|c| c.len()).unwrap_or(0)
        );
    }

    /// Handles a new prompt registration from the broker.
    pub fn on_prompt_registered(
        &self,
        name: String,
        description: String,
        arguments_schema: String,
        requires_memory: bool,
        memory_query: String,
        entity_filter: String,
    ) {
        let entry = PromptCatalogEntry {
            name,
            description,
            arguments_schema,
            requires_memory,
            memory_query,
            entity_filter,
        };
        if let Ok(mut catalog) = self.prompt_catalog.write() {
            catalog.retain(|p| p.name != entry.name);
            catalog.push(entry);
        }
        if let Ok(router) = self.prompt_router.read() {
            router.mark_dirty();
        }
        debug!(
            "Voice Assistant: prompt catalog updated, {} entries",
            self.prompt_catalog.read().map(|c| c.len()).unwrap_or(0)
        );
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
    ///
    /// Priority: runtime override (set via MCP tool) → file
    /// (`$XDG_CONFIG_HOME/smearor/voice-assistant-system-prompt.txt`) →
    /// embedded default.
    pub fn build_system_prompt(&self) -> String {
        const DEFAULT_PROMPT: &str = include_str!("../../../resources/voice-assistant-system-prompt.txt");

        if let Some(runtime) = self.runtime_system_prompt.read().ok().and_then(|guard| guard.clone()) {
            return runtime;
        }

        let config_path = xdg_config_path("voice-assistant-system-prompt.txt");
        match std::fs::read_to_string(&config_path) {
            Ok(content) if !content.trim().is_empty() => content,
            _ => DEFAULT_PROMPT.to_string(),
        }
    }

    /// Builds the dynamic context message injected as the first user message.
    /// This changes per command but does NOT trigger a worker reset.
    /// Contains: semantically selected tools, available resources, prompts, entity states,
    /// recalled long-term facts, and prompt-driven memory context.
    pub fn build_context_message(&self, user_text: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        let selected_tools = self.select_tools_for_prompt(user_text);
        let tools_json = self.serialize_tools(&selected_tools);
        parts.push(format!("Available tools: {tools_json}"));

        let selected_resources = self.select_resources_for_prompt(user_text);
        let resources_json = if selected_resources.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", selected_resources.join(","))
        };
        parts.push(format!("Available resources: {resources_json}"));

        let selected_prompts = self.select_prompts_for_prompt(user_text);
        let prompts_json = if selected_prompts.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", selected_prompts.join(","))
        };
        if prompts_json != "[]" {
            parts.push(format!("Available prompts: {prompts_json}"));
        }

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

        let prompt_memory = self.build_prompt_memory_context();
        if !prompt_memory.is_empty() {
            parts.push(prompt_memory);
        }

        let personalization_context = self.build_personalization_context();
        if !personalization_context.is_empty() {
            parts.push(personalization_context);
        }

        parts.join("\n")
    }

    /// Builds a personalization context string from the latest PersonalizationStatusMessage.
    /// Injects locale, timezone, and coordinates so the LLM can produce
    /// locale-aware responses and use the correct location for queries.
    fn build_personalization_context(&self) -> String {
        let guard = self.personalization.read().unwrap_or_else(|e| e.into_inner());
        let Some(ref status) = *guard else {
            return String::new();
        };

        let mut lines: Vec<String> = Vec::new();

        if let Some(locale) = status.locale.as_ref() {
            let locale_str = locale.to_string();
            let language_hint = match locale_str.split('-').next().unwrap_or("en") {
                "de" => "German",
                "fr" => "French",
                "es" => "Spanish",
                "it" => "Italian",
                _ => "English",
            };
            lines.push(format!("User locale: {locale_str} (respond in {language_hint})"));
        }

        if let Some(timezone) = status.timezone.as_ref() {
            lines.push(format!("User timezone: {}", timezone.to_string()));
        }

        if let Some(coords) = status.coordinates.as_ref() {
            lines.push(format!("User location: lat={}, lon={}", coords.latitude, coords.longitude));
        }

        if lines.is_empty() {
            String::new()
        } else {
            format!("User personalization:\n{}", lines.join("\n"))
        }
    }

    /// Selects tools for the context message using semantic embedding similarity.
    /// Performs a lazy rebuild if the router is dirty (tools were registered since last rebuild).
    fn select_tools_for_prompt(&self, user_text: &str) -> Vec<ToolCatalogEntry> {
        let start = std::time::Instant::now();
        // Lazy rebuild: if tools were registered since the last rebuild, rebuild now.
        if let Ok(router) = self.tool_router.read() {
            if router.is_dirty() {
                drop(router);
                self.rebuild_tool_router();
            }
        }
        let threshold = self.tool_selection_threshold.read().map(|t| *t).unwrap_or(self.config.tool_selection_threshold);
        let (result, ranking) = if let Ok(router) = self.tool_router.read() {
            router.select_tools_with_ranking(user_text, self.config.max_tools_in_prompt, threshold)
        } else {
            (Vec::new(), Vec::new())
        };
        if let Ok(mut ranking_guard) = self.last_tool_ranking.write() {
            *ranking_guard = ranking;
        }
        self.performance_monitor.record_tool_selection(start.elapsed());
        result
    }

    /// Selects resources for the context message using semantic embedding similarity.
    /// Performs a lazy rebuild if the router is dirty (resources were registered since last rebuild).
    fn select_resources_for_prompt(&self, user_text: &str) -> Vec<String> {
        if let Ok(router) = self.resource_router.read() {
            if router.is_dirty() {
                drop(router);
                self.rebuild_resource_router();
            }
        }
        let threshold = self.tool_selection_threshold.read().map(|t| *t).unwrap_or(self.config.tool_selection_threshold);
        if let Ok(router) = self.resource_router.read() {
            let (selected, ranking) = router.select_with_ranking(user_text, self.config.max_resources_in_prompt, threshold);
            if let Ok(mut ranking_guard) = self.last_resource_ranking.write() {
                *ranking_guard = ranking;
            }
            selected
        } else {
            Vec::new()
        }
    }

    /// Selects prompts for the context message using semantic embedding similarity.
    /// Performs a lazy rebuild if the router is dirty (prompts were registered since last rebuild).
    fn select_prompts_for_prompt(&self, user_text: &str) -> Vec<String> {
        if let Ok(router) = self.prompt_router.read() {
            if router.is_dirty() {
                drop(router);
                self.rebuild_prompt_router();
            }
        }
        let threshold = self.tool_selection_threshold.read().map(|t| *t).unwrap_or(self.config.tool_selection_threshold);
        if let Ok(router) = self.prompt_router.read() {
            let (selected, ranking) = router.select_with_ranking(user_text, self.config.max_prompts_in_prompt, threshold);
            if let Ok(mut ranking_guard) = self.last_prompt_ranking.write() {
                *ranking_guard = ranking;
            }
            selected
        } else {
            Vec::new()
        }
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

    /// Builds memory context for prompts that require memory access.
    /// For each prompt with `requires_memory = true`:
    /// - Queries SemanticMemory with the prompt's `memory_query`.
    /// - Filters EntityStore entries by the prompt's `entity_filter`.
    /// - Applies a token budget to prevent context overflow when many
    ///   memory-requiring prompts are active simultaneously.
    /// Empty results (no matching facts or entities) produce no header line,
    /// avoiding token waste.
    fn build_prompt_memory_context(&self) -> String {
        let prompts = match self.prompt_catalog.read() {
            Ok(prompts) => prompts,
            Err(_) => return String::new(),
        };

        let memory_prompts: Vec<&PromptCatalogEntry> = prompts.iter().filter(|p| p.requires_memory).collect();

        if memory_prompts.is_empty() {
            return String::new();
        }

        const MAX_PROMPT_MEMORY_BLOCKS: usize = 5;
        const MAX_FACTS_PER_PROMPT: usize = 3;

        let mut blocks: Vec<String> = Vec::new();

        for prompt in memory_prompts.iter().take(MAX_PROMPT_MEMORY_BLOCKS) {
            if !prompt.memory_query.is_empty() {
                let facts = self
                    .semantic_memory
                    .write()
                    .ok()
                    .and_then(|mut memory| memory.recall(&prompt.memory_query, MAX_FACTS_PER_PROMPT).ok())
                    .unwrap_or_default();

                if !facts.is_empty() {
                    let facts_summary = facts
                        .iter()
                        .map(|f| format!("- {} ({}): {}", f.key, f.category, f.value))
                        .collect::<Vec<_>>()
                        .join("\n");
                    blocks.push(format!("Prompt '{}' recalled facts:\n{facts_summary}", prompt.name));
                }
            }

            if !prompt.entity_filter.is_empty() {
                let filters: Vec<&str> = prompt.entity_filter.split(',').map(str::trim).collect();
                let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
                let filtered: Vec<String> = store
                    .values()
                    .filter(|state| filters.iter().any(|f| state.name.to_lowercase().contains(f)))
                    .map(|state| format!("- {}: {}", state.name, state.state))
                    .collect();

                if !filtered.is_empty() {
                    blocks.push(format!("Prompt '{}' relevant entities:\n{}", prompt.name, filtered.join("\n")));
                }
            }
        }

        if blocks.is_empty() {
            return String::new();
        }

        let mut result = format!("Prompt memory context:\n{}", blocks.join("\n"));

        if memory_prompts.len() > MAX_PROMPT_MEMORY_BLOCKS {
            result.push_str(&format!("\n(Truncated: {} of {} memory prompts shown)", MAX_PROMPT_MEMORY_BLOCKS, memory_prompts.len()));
        }

        result
    }
}
