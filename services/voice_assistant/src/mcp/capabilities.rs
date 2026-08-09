use crate::service::VoiceAssistantService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_voice_assistant_model::MemoryForgetArgs;
use smearor_voice_assistant_model::MemoryListArgs;
use smearor_voice_assistant_model::MemoryQueryArgs;
use smearor_voice_assistant_model::MemoryRecallArgs;
use smearor_voice_assistant_model::MemoryStoreArgs;
use smearor_voice_assistant_model::MemoryStoreBatchArgs;
use smearor_voice_assistant_model::ResourceDiscoveryGuideArgs;
use smearor_voice_assistant_model::VoiceAssistantSaveSystemPromptArgs;
use smearor_voice_assistant_model::VoiceAssistantSetMaxTokensArgs;
use smearor_voice_assistant_model::VoiceAssistantSetRollingWindowArgs;
use smearor_voice_assistant_model::VoiceAssistantSetSystemPromptArgs;
use smearor_voice_assistant_model::VoiceAssistantSetThresholdArgs;
use smearor_voice_assistant_model::VoiceAssistantSetWakeWordModelArgs;
use smearor_voice_assistant_model::VoiceAssistantSpeakArgs;
use smearor_voice_assistant_model::VoiceAssistantSubmitTextArgs;
use smearor_voice_assistant_model::VoiceAssistantSwitchModelArgs;
use smearor_voice_assistant_model::VoiceAssistantTrainingGetArgs;
use smearor_voice_assistant_model::VoiceAssistantTrainingStartArgs;
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

        let prompt_catalog_resource = RegisterResourceMessage::new(
            "voice_assistant://prompt_catalog",
            "Voice Assistant Prompt Catalog",
            "List of all discovered prompts in the catalog with memory integration metadata.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(prompt_catalog_resource);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();

        let activate_tool = RegisterToolMessage::new("voice_assistant_activate", "Starts audio capture and begins the voice pipeline.", &no_args_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(activate_tool);

        let deactivate_tool = RegisterToolMessage::new("voice_assistant_deactivate", "Stops audio capture and cancels the pipeline.", &no_args_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(deactivate_tool);

        let submit_text_schema = serde_json::to_string(&schema_for!(VoiceAssistantSubmitTextArgs)).unwrap_or_default();
        let submit_text_tool = RegisterToolMessage::new("voice_assistant_submit_text", "Submits a text command directly (bypassing STT).", &submit_text_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(submit_text_tool);

        let entities_resource = RegisterResourceMessage::new(
            "memory://entities",
            "Entity States",
            "Current states of all tracked entities (devices, apps, media).",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(entities_resource);

        let memory_query_schema = serde_json::to_string(&schema_for!(MemoryQueryArgs)).unwrap_or_default();
        let memory_query_tool = RegisterToolMessage::new(
            "memory_query",
            "Queries a specific entity by name or tool name from the entity state store.",
            &memory_query_schema,
        )
        .with_annotations(&ToolAnnotations::read_only());
        broadcaster.broadcast_message_to_topic(memory_query_tool);

        let memory_store_schema = serde_json::to_string(&schema_for!(MemoryStoreArgs)).unwrap_or_default();
        let memory_store_tool = RegisterToolMessage::new("memory_store", "Stores a fact in long-term semantic memory.", &memory_store_schema)
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(memory_store_tool);

        let memory_store_batch_schema = serde_json::to_string(&schema_for!(MemoryStoreBatchArgs)).unwrap_or_default();
        let memory_store_batch_tool = RegisterToolMessage::new(
            "memory_store_batch",
            "Stores multiple facts in long-term semantic memory in a single batch call.",
            &memory_store_batch_schema,
        )
        .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(memory_store_batch_tool);

        let memory_recall_schema = serde_json::to_string(&schema_for!(MemoryRecallArgs)).unwrap_or_default();
        let memory_recall_tool =
            RegisterToolMessage::new("memory_recall", "Recalls facts from long-term semantic memory by semantic similarity.", &memory_recall_schema)
                .with_annotations(&ToolAnnotations::read_only());
        broadcaster.broadcast_message_to_topic(memory_recall_tool);

        let memory_list_schema = serde_json::to_string(&schema_for!(MemoryListArgs)).unwrap_or_default();
        let memory_list_tool = RegisterToolMessage::new("memory_list", "Lists all stored fact keys in long-term memory.", &memory_list_schema)
            .with_annotations(&ToolAnnotations::read_only());
        broadcaster.broadcast_message_to_topic(memory_list_tool);

        let memory_forget_schema = serde_json::to_string(&schema_for!(MemoryForgetArgs)).unwrap_or_default();
        let memory_forget_tool = RegisterToolMessage::new("memory_forget", "Deletes a fact from long-term memory by key.", &memory_forget_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(memory_forget_tool);

        let switch_model_schema = serde_json::to_string(&schema_for!(VoiceAssistantSwitchModelArgs)).unwrap_or_default();
        let switch_model_tool = RegisterToolMessage::new(
            "voice_assistant_switch_model",
            "Switches the LLM model at runtime by loading a new GGUF file. The KV cache is cleared and the conversation history is preserved.",
            &switch_model_schema,
        )
        .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(switch_model_tool);

        let voice_assistant_prompt = RegisterPromptMessage::with_memory(
            "voice_assistant_status",
            "Returns the current voice assistant status including state, transcript, and final answer.",
            &no_args_schema,
            "voice assistant usage preferences and interaction history",
            "voice",
        );
        broadcaster.broadcast_message_to_topic(voice_assistant_prompt);

        let memory_prompt = RegisterPromptMessage::with_memory(
            "memory_guide",
            "Returns a system prompt with memory management instructions for the voice assistant.",
            &no_args_schema,
            "memory management preferences and frequently recalled facts",
            "memory",
        );
        broadcaster.broadcast_message_to_topic(memory_prompt);

        let resource_discovery_schema = serde_json::to_string(&schema_for!(ResourceDiscoveryGuideArgs)).unwrap_or_default();
        let resource_discovery_prompt = RegisterPromptMessage::with_memory(
            "resource_discovery_guide",
            "Returns a dynamic list of available MCP resources, their URIs, and usage instructions. Use this when the user asks for resources or how to read system data.",
            &resource_discovery_schema,
            "resource discovery preferences and frequently accessed resources",
            "resource",
        );
        broadcaster.broadcast_message_to_topic(resource_discovery_prompt);

        let training_start_schema = serde_json::to_string(&schema_for!(VoiceAssistantTrainingStartArgs)).unwrap_or_default();
        let training_start_tool = RegisterToolMessage::new(
            "voice_assistant_training_start",
            "Enables training mode for the Voice Assistant. The next user interaction will be recorded as a training trace.",
            &training_start_schema,
        )
        .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(training_start_tool);

        let training_end_tool =
            RegisterToolMessage::new("voice_assistant_training_end", "Disables training mode and finalizes the current trace.", &no_args_schema)
                .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(training_end_tool);

        let training_get_schema = serde_json::to_string(&schema_for!(VoiceAssistantTrainingGetArgs)).unwrap_or_default();
        let training_get_tool = RegisterToolMessage::new(
            "voice_assistant_training_get",
            "Returns the last N recorded training traces, optionally filtered by label or user text substring.",
            &training_get_schema,
        )
        .with_annotations(&ToolAnnotations::read_only());
        broadcaster.broadcast_message_to_topic(training_get_tool);

        let set_threshold_schema = serde_json::to_string(&schema_for!(VoiceAssistantSetThresholdArgs)).unwrap_or_default();
        let set_threshold_tool = RegisterToolMessage::new(
            "voice_assistant_set_threshold",
            "Sets the tool selection threshold at runtime. The threshold (0.0–1.0) controls the minimum cosine similarity for tools, resources, and prompts to be included in the LLM context. Lower values include more tools, higher values are more selective.",
            &set_threshold_schema,
        )
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(set_threshold_tool);

        let set_rolling_window_schema = serde_json::to_string(&schema_for!(VoiceAssistantSetRollingWindowArgs)).unwrap_or_default();
        let set_rolling_window_tool = RegisterToolMessage::new(
            "voice_assistant_set_rolling_window",
            "Sets the rolling window keep-last parameter at runtime. This controls how many trailing conversation messages (tool calls and responses) are preserved during context overflow trimming. Each tool call/response pair is 2 messages. The context message (tool schemas) is always preserved in addition.",
            &set_rolling_window_schema,
        )
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(set_rolling_window_tool);

        let set_max_tokens_schema = serde_json::to_string(&schema_for!(VoiceAssistantSetMaxTokensArgs)).unwrap_or_default();
        let set_max_tokens_tool = RegisterToolMessage::new(
            "voice_assistant_set_max_tokens",
            "Sets the max tokens for LLM generation at runtime without reloading the model. Higher values allow longer responses but may increase latency. The change takes effect on the next LLM generation call.",
            &set_max_tokens_schema,
        )
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(set_max_tokens_tool);

        let clear_conversation_tool = RegisterToolMessage::new(
            "voice_assistant_clear_conversation",
            "Clears the conversation history and LLM KV cache. Use this between test runs to prevent context contamination from previous interactions.",
            &no_args_schema,
        )
        .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(clear_conversation_tool);

        let get_system_prompt_tool = RegisterToolMessage::new(
            "voice_assistant_get_system_prompt",
            "Returns the current system prompt used by the voice assistant LLM. The prompt is loaded from voice-assistant-system-prompt.txt or a runtime override.",
            &no_args_schema,
        )
            .with_annotations(&ToolAnnotations::read_only());
        broadcaster.broadcast_message_to_topic(get_system_prompt_tool);

        let set_system_prompt_schema = serde_json::to_string(&schema_for!(VoiceAssistantSetSystemPromptArgs)).unwrap_or_default();
        let set_system_prompt_tool = RegisterToolMessage::new(
            "voice_assistant_set_system_prompt",
            "Sets a runtime override for the voice assistant system prompt. Pass the full prompt text. Pass an empty string to clear the override and revert to the file-based prompt. The change takes effect on the next ReAct loop.",
            &set_system_prompt_schema,
        )
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(set_system_prompt_tool);

        let save_system_prompt_schema = serde_json::to_string(&schema_for!(VoiceAssistantSaveSystemPromptArgs)).unwrap_or_default();
        let save_system_prompt_tool = RegisterToolMessage::new(
            "voice_assistant_save_system_prompt",
            "Saves the given system prompt text to voice-assistant-system-prompt.txt, persisting it to disk. This permanently updates the file-based prompt. Pass the full prompt text to write.",
            &save_system_prompt_schema,
        )
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(save_system_prompt_tool);

        let enable_wake_word_tool = RegisterToolMessage::new(
            "voice_assistant_enable_wake_word",
            "Enables wake word detection mode. The assistant enters a Standby state and continuously listens for a wake word using openWakeWord. When detected, the voice pipeline is automatically activated.",
            &no_args_schema,
        )
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(enable_wake_word_tool);

        let disable_wake_word_tool = RegisterToolMessage::new(
            "voice_assistant_disable_wake_word",
            "Disables wake word detection mode. Stops the wake word detector and shared audio source, returning the assistant to Idle state.",
            &no_args_schema,
        )
        .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(disable_wake_word_tool);

        let set_wake_word_model_schema = serde_json::to_string(&schema_for!(VoiceAssistantSetWakeWordModelArgs)).unwrap_or_default();
        let set_wake_word_model_tool = RegisterToolMessage::new(
            "voice_assistant_set_wake_word_model",
            "Changes the wake word model and/or detection threshold at runtime. If wake word detection is currently active, the detector is automatically restarted with the new settings. Supported models: Alexa, HeyMycroft, Custom.",
            &set_wake_word_model_schema,
        )
            .with_annotations(&ToolAnnotations::idempotent());
        broadcaster.broadcast_message_to_topic(set_wake_word_model_tool);

        let speak_schema = serde_json::to_string(&schema_for!(VoiceAssistantSpeakArgs)).unwrap_or_default();
        let speak_tool = RegisterToolMessage::new(
            "voice_assistant_speak",
            "Speaks the given text directly via TTS, bypassing the LLM. The text is not processed by the voice pipeline — it is synthesized and played back immediately.",
            &speak_schema,
        )
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(speak_tool);
    }
}
