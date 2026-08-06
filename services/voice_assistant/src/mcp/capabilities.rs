use crate::service::VoiceAssistantService;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use tracing::debug;

impl McpCapabilitiesRegistrator for VoiceAssistantService {
    fn register_mcp_capabilities(&self) {
        if !self.config.mcp_enabled {
            debug!("Voice Assistant Service: MCP tool registration disabled by config");
            return;
        }

        let broadcaster = self.get_broadcaster();

        let status_resource = RegisterResourceMessage::new(
            "voice_assistant://status",
            "Voice Assistant Status",
            "Current assistant state, transcript, and final answer.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(status_resource);

        let tool_catalog_resource = RegisterResourceMessage::new(
            "voice_assistant://tool_catalog",
            "Voice Assistant Tool Catalog",
            "List of all discovered tools in the catalog.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(tool_catalog_resource);

        let llm_resource = RegisterResourceMessage::new(
            "voice_assistant://llm",
            "Voice Assistant LLM Configuration",
            "Current LLM model, context size, max tokens, temperature, and other inference parameters.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(llm_resource);

        let stt_resource = RegisterResourceMessage::new(
            "voice_assistant://stt",
            "Voice Assistant STT Configuration",
            "Speech-to-Text configuration: Whisper model, sample rate, silence detection, and language.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(stt_resource);

        let tts_resource = RegisterResourceMessage::new(
            "voice_assistant://tts",
            "Voice Assistant TTS Configuration",
            "Text-to-Speech configuration: model, type, sample rate, language, and voice.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(tts_resource);

        let embeddings_resource = RegisterResourceMessage::new(
            "voice_assistant://embeddings",
            "Voice Assistant Embeddings Configuration",
            "Embedding engine status: model name, fallback flag, cache stats, tool selection threshold, and execution provider.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(embeddings_resource);

        let models_resource = RegisterResourceMessage::new(
            "voice_assistant://models",
            "Voice Assistant Available Models",
            "Lists all GGUF model files available in the models/ directory for runtime model switching.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(models_resource);

        let activate_tool = RegisterToolMessage::new(
            "voice_assistant_activate",
            "Starts audio capture and begins the voice pipeline.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(activate_tool);

        let deactivate_tool = RegisterToolMessage::new(
            "voice_assistant_deactivate",
            "Stops audio capture and cancels the pipeline.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(deactivate_tool);

        let submit_text_tool = RegisterToolMessage::new(
            "voice_assistant_submit_text",
            "Submits a text command directly (bypassing STT).",
            r#"{ "type": "object", "properties": { "text": { "type": "string", "description": "The text command to submit" } }, "required": ["text"] }"#,
        );
        broadcaster.broadcast_message_to_topic(submit_text_tool);

        let entities_resource = RegisterResourceMessage::new(
            "memory://entities",
            "Entity States",
            "Current states of all tracked entities (devices, apps, media).",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(entities_resource);

        let memory_query_tool = RegisterToolMessage::new(
            "memory_query",
            "Queries a specific entity by name or tool name from the entity state store.",
            r#"{ "type": "object", "properties": { "query": { "type": "string", "description": "Entity name or tool name to look up" } }, "required": ["query"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_query_tool);

        let memory_store_tool = RegisterToolMessage::new(
            "memory_store",
            "Stores a fact in long-term semantic memory.",
            r#"{ "type": "object", "properties": { "key": { "type": "string", "description": "Short key for the fact" }, "value": { "type": "string", "description": "The fact content" }, "category": { "type": "string", "description": "Category: fact, preference, or habit" } }, "required": ["key", "value"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_store_tool);

        let memory_store_batch_tool = RegisterToolMessage::new(
            "memory_store_batch",
            "Stores multiple facts in long-term semantic memory in a single batch call.",
            r#"{ "type": "object", "properties": { "facts": { "type": "array", "items": { "type": "object", "properties": { "key": { "type": "string", "description": "Short key for the fact" }, "value": { "type": "string", "description": "The fact content" }, "category": { "type": "string", "description": "Category: fact, preference, or habit" } }, "required": ["key", "value"] }, "description": "Array of facts to store" } }, "required": ["facts"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_store_batch_tool);

        let memory_recall_tool = RegisterToolMessage::new(
            "memory_recall",
            "Recalls facts from long-term semantic memory by semantic similarity.",
            r#"{ "type": "object", "properties": { "query": { "type": "string", "description": "Natural language query to find related facts" }, "limit": { "type": "integer", "description": "Max number of facts to return (default: 3)" } }, "required": ["query"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_recall_tool);

        let memory_list_tool = RegisterToolMessage::new(
            "memory_list",
            "Lists all stored fact keys in long-term memory.",
            r#"{ "type": "object", "properties": { "category": { "type": "string", "description": "Optional category filter: fact, preference, or habit" } }, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_list_tool);

        let memory_forget_tool = RegisterToolMessage::new(
            "memory_forget",
            "Deletes a fact from long-term memory by key.",
            r#"{ "type": "object", "properties": { "key": { "type": "string", "description": "The key of the fact to delete" } }, "required": ["key"] }"#,
        );
        broadcaster.broadcast_message_to_topic(memory_forget_tool);

        let switch_model_tool = RegisterToolMessage::new(
            "voice_assistant_switch_model",
            "Switches the LLM model at runtime by loading a new GGUF file. The KV cache is cleared and the conversation history is preserved.",
            r#"{ "type": "object", "properties": { "model_path": { "type": "string", "description": "Path to the new GGUF model file (e.g., 'models/qwen2.5-7b-instruct-q4_k_m.gguf')" }, "n_ctx": { "type": "integer", "description": "Override the context window size (e.g., 4096 for larger models with limited VRAM). Omit to use the configured default." }, "max_tokens": { "type": "integer", "description": "Override the max tokens to generate per response. Omit to use the default of 512." } }, "required": ["model_path"] }"#,
        );
        broadcaster.broadcast_message_to_topic(switch_model_tool);

        let voice_assistant_prompt = RegisterPromptMessage::with_memory(
            "voice_assistant_status",
            "Returns the current voice assistant status including state, transcript, and final answer.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
            "voice assistant usage preferences and interaction history",
            "voice",
        );
        broadcaster.broadcast_message_to_topic(voice_assistant_prompt);

        let memory_prompt = RegisterPromptMessage::with_memory(
            "memory_guide",
            "Returns a system prompt with memory management instructions for the voice assistant.",
            r#"{ "type": "object", "properties": {} }"#,
            "memory management preferences and frequently recalled facts",
            "memory",
        );
        broadcaster.broadcast_message_to_topic(memory_prompt);

        let resource_discovery_prompt = RegisterPromptMessage::with_memory(
            "resource_discovery_guide",
            "Returns a dynamic list of available MCP resources, their URIs, and usage instructions. Use this when the user asks for resources or how to read system data.",
            r#"{ "type": "object", "properties": { "filter": { "type": "string", "description": "Optional keyword to filter resources by category" } } }"#,
            "resource discovery preferences and frequently accessed resources",
            "resource",
        );
        broadcaster.broadcast_message_to_topic(resource_discovery_prompt);

        let training_start_tool = RegisterToolMessage::new(
            "voice_assistant_training_start",
            "Enables training mode for the Voice Assistant. The next user interaction will be recorded as a training trace.",
            r#"{ "type": "object", "properties": { "label": { "type": "string", "description": "Optional label for the training trace (e.g. 'favorite_song_test')" } }, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(training_start_tool);

        let training_end_tool = RegisterToolMessage::new(
            "voice_assistant_training_end",
            "Disables training mode and finalizes the current trace.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(training_end_tool);

        let training_get_tool = RegisterToolMessage::new(
            "voice_assistant_training_get",
            "Returns the last N recorded training traces, optionally filtered by label or user text substring.",
            r#"{ "type": "object", "properties": { "limit": { "type": "integer", "description": "Maximum number of traces to return (default: 1)" }, "label": { "type": "string", "description": "Optional label to filter traces" }, "query": { "type": "string", "description": "Optional substring to search in user_text" }, "trace_id": { "type": "string", "description": "Optional specific trace ID to retrieve" } }, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(training_get_tool);

        let set_threshold_tool = RegisterToolMessage::new(
            "voice_assistant_set_threshold",
            "Sets the tool selection threshold at runtime. The threshold (0.0–1.0) controls the minimum cosine similarity for tools, resources, and prompts to be included in the LLM context. Lower values include more tools, higher values are more selective.",
            r#"{ "type": "object", "properties": { "threshold": { "type": "number", "description": "Minimum cosine similarity (0.0–1.0). Default: 0.3. Lower = more tools, higher = fewer tools." } }, "required": ["threshold"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_threshold_tool);

        let set_rolling_window_tool = RegisterToolMessage::new(
            "voice_assistant_set_rolling_window",
            "Sets the rolling window keep-last parameter at runtime. This controls how many trailing conversation messages (tool calls and responses) are preserved during context overflow trimming. Each tool call/response pair is 2 messages. The context message (tool schemas) is always preserved in addition.",
            r#"{ "type": "object", "properties": { "keep_last": { "type": "integer", "description": "Number of trailing messages to keep (default: 6, i.e. 3 tool-call/response pairs). Minimum: 2." } }, "required": ["keep_last"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_rolling_window_tool);

        let set_max_tokens_tool = RegisterToolMessage::new(
            "voice_assistant_set_max_tokens",
            "Sets the max tokens for LLM generation at runtime without reloading the model. Higher values allow longer responses but may increase latency. The change takes effect on the next LLM generation call.",
            r#"{ "type": "object", "properties": { "max_tokens": { "type": "integer", "description": "Maximum number of tokens to generate per LLM response (default: 512). Typical range: 256–2048." } }, "required": ["max_tokens"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_max_tokens_tool);

        let clear_conversation_tool = RegisterToolMessage::new(
            "voice_assistant_clear_conversation",
            "Clears the conversation history and LLM KV cache. Use this between test runs to prevent context contamination from previous interactions.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(clear_conversation_tool);

        let get_system_prompt_tool = RegisterToolMessage::new(
            "voice_assistant_get_system_prompt",
            "Returns the current system prompt used by the voice assistant LLM. The prompt is loaded from voice-assistant-system-prompt.txt or a runtime override.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(get_system_prompt_tool);

        let set_system_prompt_tool = RegisterToolMessage::new(
            "voice_assistant_set_system_prompt",
            "Sets a runtime override for the voice assistant system prompt. Pass the full prompt text. Pass an empty string to clear the override and revert to the file-based prompt. The change takes effect on the next ReAct loop.",
            r#"{ "type": "object", "properties": { "system_prompt": { "type": "string", "description": "The full system prompt text. Pass empty string to clear the override." } }, "required": ["system_prompt"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_system_prompt_tool);

        let save_system_prompt_tool = RegisterToolMessage::new(
            "voice_assistant_save_system_prompt",
            "Saves the given system prompt text to voice-assistant-system-prompt.txt, persisting it to disk. This permanently updates the file-based prompt. Pass the full prompt text to write.",
            r#"{ "type": "object", "properties": { "system_prompt": { "type": "string", "description": "The full system prompt text to save to the file." } }, "required": ["system_prompt"] }"#,
        );
        broadcaster.broadcast_message_to_topic(save_system_prompt_tool);

        let enable_wake_word_tool = RegisterToolMessage::new(
            "voice_assistant_enable_wake_word",
            "Enables wake word detection mode. The assistant enters a Standby state and continuously listens for a wake word using openWakeWord. When detected, the voice pipeline is automatically activated.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(enable_wake_word_tool);

        let disable_wake_word_tool = RegisterToolMessage::new(
            "voice_assistant_disable_wake_word",
            "Disables wake word detection mode. Stops the wake word detector and shared audio source, returning the assistant to Idle state.",
            r#"{ "type": "object", "properties": {}, "required": [] }"#,
        );
        broadcaster.broadcast_message_to_topic(disable_wake_word_tool);

        let set_wake_word_model_tool = RegisterToolMessage::new(
            "voice_assistant_set_wake_word_model",
            "Changes the wake word model and/or detection threshold at runtime. If wake word detection is currently active, the detector is automatically restarted with the new settings. Supported models: Alexa, HeyMycroft, Custom.",
            r#"{ "type": "object", "properties": { "model": { "type": "string", "description": "Wake word model name: Alexa, HeyMycroft, or Custom" }, "threshold": { "type": "number", "description": "Detection threshold (0.0-1.0). Lower = more sensitive. Optional." } }, "required": ["model"] }"#,
        );
        broadcaster.broadcast_message_to_topic(set_wake_word_model_tool);

        let speak_tool = RegisterToolMessage::new(
            "voice_assistant_speak",
            "Speaks the given text directly via TTS, bypassing the LLM. The text is not processed by the voice pipeline — it is synthesized and played back immediately.",
            r#"{ "type": "object", "properties": { "text": { "type": "string", "description": "The text to speak via TTS" } }, "required": ["text"] }"#,
        );
        broadcaster.broadcast_message_to_topic(speak_tool);
    }
}
