use crate::config::ContextConfig;
use crate::llm_backend::LlmBackendConfig;
use crate::memory::FactCategory;
use crate::service::VoiceAssistantService;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_voice_assistant_model::MemoryStoreBatchArgs;
use smearor_voice_assistant_model::VoiceAssistantMcpTools;
use smearor_voice_assistant_model::VoiceAssistantSaveSystemPromptArgs;
use smearor_voice_assistant_model::VoiceAssistantSetMaxTokensArgs;
use smearor_voice_assistant_model::VoiceAssistantSetRollingWindowArgs;
use smearor_voice_assistant_model::VoiceAssistantSetSystemPromptArgs;
use smearor_voice_assistant_model::VoiceAssistantSetThresholdArgs;
use smearor_voice_assistant_model::VoiceAssistantSetWakeWordModelArgs;
use smearor_voice_assistant_model::VoiceAssistantSpeakArgs;
use smearor_voice_assistant_model::VoiceAssistantSwitchModelArgs;
use smearor_voice_assistant_model::VoiceAssistantTrainingGetArgs;
use smearor_voice_assistant_model::VoiceAssistantTrainingStartArgs;
use std::str::FromStr;
use tracing::debug;
use tracing::error;

impl MessageHandler<FfiEnvelopePayload<InvokeToolMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeToolMessage>, _sender_id: &str) {
        let tool_name = message.0.name.to_string();
        debug!("Voice Assistant Service: InvokeToolMessage name={}", tool_name);
        let broadcaster = self.get_broadcaster();
        let tool = match VoiceAssistantMcpTools::from_str(&tool_name) {
            Ok(tool) => tool,
            Err(_) => {
                debug!("Voice Assistant Service: ignoring InvokeToolMessage for external tool '{tool_name}' (handled by launcher core)");
                return;
            }
        };
        match tool {
            VoiceAssistantMcpTools::Activate => {
                self.activate();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Voice assistant activated");
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::Deactivate => {
                self.deactivate();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Voice assistant deactivated");
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::SubmitText => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let text = args.get("text").and_then(|v| v.as_str());
                match text {
                    Some(text) => {
                        self.submit_text(text);
                        let response = InvokeToolResponse::success(&message.0.correlation_id, "Text submitted");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: text");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            VoiceAssistantMcpTools::MemoryQuery => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
                let result = store
                    .iter()
                    .find(|(_, state)| state.name.to_lowercase().contains(&query.to_lowercase()) || state.tool.to_lowercase().contains(&query.to_lowercase()))
                    .map(|(_, state)| serde_json::to_string(state).unwrap_or_default());
                match result {
                    Some(json) => {
                        let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Entity not found: {query}"));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            VoiceAssistantMcpTools::MemoryStore => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let category_str = args.get("category").and_then(|v| v.as_str()).unwrap_or("fact");
                let category = category_str.parse().unwrap_or(FactCategory::Fact);
                if key.is_empty() || value.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameters: key and value");
                    broadcaster.broadcast_message_to_topic(response);
                } else if let Ok(mut memory) = self.semantic_memory.write() {
                    match memory.store(key, value, category) {
                        Ok(id) => {
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Fact stored with id: {id}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Store failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            VoiceAssistantMcpTools::MemoryRecall => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
                if let Ok(mut memory) = self.semantic_memory.write() {
                    match memory.recall(query, limit) {
                        Ok(facts) => {
                            let json = serde_json::to_string(
                                &facts
                                    .iter()
                                    .map(|f| serde_json::json!({"key": f.key, "value": f.value, "category": f.category.to_string()}))
                                    .collect::<Vec<_>>(),
                            )
                            .unwrap_or_default();
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Recall failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            VoiceAssistantMcpTools::MemoryList => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let category_str = args.get("category").and_then(|v| v.as_str());
                let category = category_str.and_then(|s| s.parse().ok());
                if let Ok(memory) = self.semantic_memory.read() {
                    match memory.list_keys(category.as_ref()) {
                        Ok(keys) => {
                            let json = serde_json::to_string(&keys).unwrap_or_default();
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("List failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            VoiceAssistantMcpTools::MemoryForget => {
                let args: serde_json::Value = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                if key.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: key");
                    broadcaster.broadcast_message_to_topic(response);
                } else if let Ok(memory) = self.semantic_memory.read() {
                    match memory.forget(key) {
                        Ok(()) => {
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Fact '{key}' forgotten"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Forget failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            VoiceAssistantMcpTools::MemoryStoreBatch => {
                let args: MemoryStoreBatchArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                if args.facts.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: facts (non-empty array)");
                    broadcaster.broadcast_message_to_topic(response);
                    return;
                }
                let facts: Vec<(String, String, FactCategory)> = args
                    .facts
                    .iter()
                    .filter_map(|item| {
                        let category = item.category.as_deref().unwrap_or("fact").parse().unwrap_or(FactCategory::Fact);
                        Some((item.key.clone(), item.value.clone(), category))
                    })
                    .collect();
                if facts.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "No valid facts in array");
                    broadcaster.broadcast_message_to_topic(response);
                } else if let Ok(mut memory) = self.semantic_memory.write() {
                    match memory.store_batch(&facts) {
                        Ok(ids) => {
                            let json = serde_json::to_string(&ids).unwrap_or_default();
                            let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Batch store failed: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                } else {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "Memory lock poisoned");
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            VoiceAssistantMcpTools::TrainingStart => {
                let args: VoiceAssistantTrainingStartArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let label = args.label;
                if let Ok(mut mode) = self.training_mode.lock() {
                    *mode = true;
                }
                let trace_id = if let Ok(mut trace) = self.active_trace.lock() {
                    let new_trace = crate::training::TrainingTrace::new("", label);
                    let id = new_trace.id.clone();
                    *trace = Some(new_trace);
                    id
                } else {
                    String::new()
                };
                debug!("Voice Assistant: training mode started, trace_id={}", trace_id);
                let json = serde_json::json!({"status": "ok", "trace_id": trace_id});
                let response = InvokeToolResponse::success(&message.0.correlation_id, &json.to_string());
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::TrainingEnd => {
                if let Ok(mut mode) = self.training_mode.lock() {
                    *mode = false;
                }
                let trace_id = if let Ok(mut trace_guard) = self.active_trace.lock() {
                    if let Some(mut trace) = trace_guard.take() {
                        trace.finalize(trace.success.unwrap_or(false));
                        let id = trace.id.clone();
                        if let Ok(mut history) = self.training_history.lock() {
                            history.insert(id.clone(), trace);
                        }
                        id
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                debug!("Voice Assistant: training mode ended, trace_id={}", trace_id);
                let json = serde_json::json!({"status": "ok", "trace_id": trace_id});
                let response = InvokeToolResponse::success(&message.0.correlation_id, &json.to_string());
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::TrainingGet => {
                let args: VoiceAssistantTrainingGetArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let limit = args.limit.unwrap_or(1) as usize;

                let traces = if let Some(ref tid) = args.trace_id {
                    crate::training::get_trace_by_id(&self.training_history, tid)
                        .or_else(|| crate::training::get_active_trace(&self.active_trace, tid))
                        .map(|t| vec![t])
                        .unwrap_or_default()
                } else {
                    crate::training::query_traces(&self.training_history, limit, args.label.as_deref(), args.query.as_deref())
                };

                let json = serde_json::to_string(&traces).unwrap_or_else(|_| "[]".to_string());
                let response = InvokeToolResponse::success(&message.0.correlation_id, &json);
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::SwitchModel => {
                let args: VoiceAssistantSwitchModelArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                match args.model_path.is_empty() {
                    false => {
                        let path = shellexpand::tilde(&args.model_path).into_owned();
                        let backend_type = args.backend.as_deref().unwrap_or("local");
                        match backend_type {
                            "ollama" => {
                                let mut ollama_config = self.config.ollama.clone();
                                ollama_config.model = path.clone();
                                if let Some(max_tokens) = args.max_tokens.map(|v| v as usize) {
                                    ollama_config.max_tokens = max_tokens;
                                }
                                match self.llm_backend.read().unwrap().clone() {
                                    Some(backend) => {
                                        let backend = backend.clone();
                                        let path_for_log = path.clone();
                                        std::thread::spawn(move || {
                                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
                                            match runtime {
                                                Ok(runtime) => {
                                                    runtime.block_on(async move {
                                                        match backend.reload_model(LlmBackendConfig::Ollama(ollama_config)).await {
                                                            Ok(()) => {
                                                                debug!("Voice Assistant: model switched to Ollama {path_for_log}");
                                                            }
                                                            Err(error) => {
                                                                error!("Voice Assistant: failed to switch Ollama model: {error}");
                                                            }
                                                        }
                                                    });
                                                }
                                                Err(error) => {
                                                    error!("Voice Assistant: failed to create runtime for model switch: {error}");
                                                }
                                            }
                                        });
                                        let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Switching Ollama model to: {path}"));
                                        broadcaster.broadcast_message_to_topic(response);
                                    }
                                    None => {
                                        let response = InvokeToolResponse::error(&message.0.correlation_id, "LLM backend not initialized");
                                        broadcaster.broadcast_message_to_topic(response);
                                    }
                                }
                            }
                            _ => {
                                if args.ensure_model.unwrap_or(false) {
                                    crate::model_downloader::ensure_model(&path, &self.config.llm_model_repo);
                                }
                                let new_llm_config =
                                    self.config
                                        .to_llm_config_with_model(&path, args.n_ctx.map(|v| v as u32), args.max_tokens.map(|v| v as usize));
                                match self.llm_backend.read().unwrap().clone() {
                                    Some(backend) => {
                                        let backend = backend.clone();
                                        let path_for_log = path.clone();
                                        std::thread::spawn(move || {
                                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
                                            match runtime {
                                                Ok(runtime) => {
                                                    runtime.block_on(async move {
                                                        match backend.reload_model(LlmBackendConfig::Local(new_llm_config)).await {
                                                            Ok(()) => {
                                                                debug!("Voice Assistant: model switched to {path_for_log}");
                                                            }
                                                            Err(error) => {
                                                                error!("Voice Assistant: failed to switch model: {error}");
                                                            }
                                                        }
                                                    });
                                                }
                                                Err(error) => {
                                                    error!("Voice Assistant: failed to create runtime for model switch: {error}");
                                                }
                                            }
                                        });
                                        let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Switching LLM model to: {path}"));
                                        broadcaster.broadcast_message_to_topic(response);
                                    }
                                    None => {
                                        let response = InvokeToolResponse::error(&message.0.correlation_id, "LLM backend not initialized");
                                        broadcaster.broadcast_message_to_topic(response);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: model_path");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            VoiceAssistantMcpTools::SetThreshold => {
                let args: VoiceAssistantSetThresholdArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let threshold = args.threshold.map(|v| v as f64);
                match threshold {
                    Some(value) if (0.0..=1.0).contains(&value) => {
                        let new_threshold = value as f32;
                        if let Ok(mut guard) = self.tool_selection_threshold.write() {
                            let old_threshold = *guard;
                            *guard = new_threshold;
                            debug!("Voice Assistant: tool selection threshold updated: {old_threshold:.2} -> {new_threshold:.2}");
                            let response =
                                InvokeToolResponse::success(&message.0.correlation_id, &format!("Tool selection threshold set to {new_threshold:.2}"));
                            broadcaster.broadcast_message_to_topic(response);
                        } else {
                            let response = InvokeToolResponse::error(&message.0.correlation_id, "Failed to acquire threshold lock");
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                    Some(value) => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Threshold must be between 0.0 and 1.0, got {value}"));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: threshold");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            VoiceAssistantMcpTools::SetRollingWindow => {
                let args: VoiceAssistantSetRollingWindowArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let keep_last = args.keep_last.map(|v| v as u64);
                match keep_last {
                    Some(value) if value >= 2 => {
                        let new_keep_last = value as usize;
                        match self.llm_backend.read().unwrap().clone() {
                            Some(backend) => {
                                let backend = backend.clone();
                                let correlation_id = message.0.correlation_id.clone();
                                let old_keep_last = self.config.rolling_window_keep_last;
                                let new_context_config = ContextConfig {
                                    rolling_window_keep_last: new_keep_last,
                                    max_context_tokens: self.config.max_context_tokens,
                                    context_keep_ratio: self.config.context_keep_ratio,
                                    min_preserve_tokens: self.config.min_preserve_tokens,
                                };
                                std::thread::spawn(move || {
                                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
                                    match runtime {
                                        Ok(runtime) => {
                                            runtime.block_on(async move {
                                                match backend.update_context_config(new_context_config).await {
                                                    Ok(()) => {
                                                        debug!("Voice Assistant: rolling window keep_last updated: {old_keep_last} -> {new_keep_last}");
                                                    }
                                                    Err(error) => {
                                                        tracing::error!("Voice Assistant: failed to update rolling window: {error}");
                                                    }
                                                }
                                            });
                                        }
                                        Err(error) => {
                                            tracing::error!("Voice Assistant: failed to create runtime for rolling window update: {error}");
                                        }
                                    }
                                });
                                let response = InvokeToolResponse::success(&correlation_id, &format!("Rolling window keep_last set to {new_keep_last}"));
                                broadcaster.broadcast_message_to_topic(response);
                            }
                            None => {
                                let response = InvokeToolResponse::error(&message.0.correlation_id, "LLM backend not initialized");
                                broadcaster.broadcast_message_to_topic(response);
                            }
                        }
                    }
                    Some(value) => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("keep_last must be >= 2, got {value}"));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: keep_last");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            VoiceAssistantMcpTools::SetMaxTokens => {
                let args: VoiceAssistantSetMaxTokensArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let max_tokens = args.max_tokens.map(|v| v as u64);
                match max_tokens {
                    Some(value) if value >= 64 && value <= 8192 => {
                        let new_max_tokens = value as usize;
                        match self.llm_backend.read().unwrap().clone() {
                            Some(backend) => {
                                let old_max_tokens = backend.max_tokens();
                                backend.set_max_tokens(new_max_tokens);
                                debug!("Voice Assistant: max_tokens updated: {old_max_tokens} -> {new_max_tokens}");
                                let response = InvokeToolResponse::success(&message.0.correlation_id, &format!("Max tokens set to {new_max_tokens}"));
                                broadcaster.broadcast_message_to_topic(response);
                            }
                            None => {
                                let response = InvokeToolResponse::error(&message.0.correlation_id, "LLM backend not initialized");
                                broadcaster.broadcast_message_to_topic(response);
                            }
                        }
                    }
                    Some(value) => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("max_tokens must be between 64 and 8192, got {value}"));
                        broadcaster.broadcast_message_to_topic(response);
                    }
                    None => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "Missing required parameter: max_tokens");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
            VoiceAssistantMcpTools::ClearConversation => {
                self.clear_conversation();
                debug!("Voice Assistant: conversation history cleared via MCP tool");
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Conversation history cleared");
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::GetSystemPrompt => {
                let prompt = self.build_system_prompt();
                debug!("Voice Assistant: get_system_prompt returned {} chars", prompt.len());
                let response = InvokeToolResponse::success(&message.0.correlation_id, &prompt);
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::SetSystemPrompt => {
                let args: VoiceAssistantSetSystemPromptArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let new_prompt = &args.system_prompt;
                if new_prompt.is_empty() {
                    if let Ok(mut guard) = self.runtime_system_prompt.write() {
                        *guard = None;
                    }
                    debug!("Voice Assistant: system prompt runtime override cleared");
                    let response = InvokeToolResponse::success(&message.0.correlation_id, "System prompt override cleared. Reverting to file-based prompt.");
                    broadcaster.broadcast_message_to_topic(response);
                } else {
                    if let Ok(mut guard) = self.runtime_system_prompt.write() {
                        *guard = Some(new_prompt.to_string());
                    }
                    debug!("Voice Assistant: system prompt runtime override set ({} chars)", new_prompt.len());
                    let response = InvokeToolResponse::success(
                        &message.0.correlation_id,
                        &format!("System prompt override set ({} chars). Takes effect on next ReAct loop.", new_prompt.len()),
                    );
                    broadcaster.broadcast_message_to_topic(response);
                }
            }
            VoiceAssistantMcpTools::SaveSystemPrompt => {
                let args: VoiceAssistantSaveSystemPromptArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let prompt_text = &args.system_prompt;
                if prompt_text.is_empty() {
                    let response = InvokeToolResponse::error(&message.0.correlation_id, "system_prompt must not be empty");
                    broadcaster.broadcast_message_to_topic(response);
                } else {
                    match std::fs::write(smearor_voice_assistant_model::xdg_config_path("voice-assistant-system-prompt.txt"), prompt_text) {
                        Ok(_) => {
                            debug!("Voice Assistant: system prompt saved to file ({} chars)", prompt_text.len());
                            if let Ok(mut guard) = self.runtime_system_prompt.write() {
                                *guard = None;
                            }
                            let response = InvokeToolResponse::success(
                                &message.0.correlation_id,
                                &format!("System prompt saved to file ({} chars). Runtime override cleared.", prompt_text.len()),
                            );
                            broadcaster.broadcast_message_to_topic(response);
                        }
                        Err(error) => {
                            debug!("Voice Assistant: failed to save system prompt: {error}");
                            let response = InvokeToolResponse::error(&message.0.correlation_id, &format!("Failed to save system prompt: {error}"));
                            broadcaster.broadcast_message_to_topic(response);
                        }
                    }
                }
            }
            VoiceAssistantMcpTools::EnableWakeWord => {
                self.enable_wake_word();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Wake word detection enabled. Assistant is now in Standby mode.");
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::DisableWakeWord => {
                self.disable_wake_word();
                let response = InvokeToolResponse::success(&message.0.correlation_id, "Wake word detection disabled. Assistant returned to Idle.");
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::SetWakeWordModel => {
                let args: VoiceAssistantSetWakeWordModelArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let model = if args.model.is_empty() { "Alexa" } else { &args.model };
                let threshold = args.threshold;
                self.set_wake_word_model(model, threshold);
                let msg = match threshold {
                    Some(t) => format!("Wake word model set to '{model}' with threshold {t}."),
                    None => format!("Wake word model set to '{model}'."),
                };
                let response = InvokeToolResponse::success(&message.0.correlation_id, &msg);
                broadcaster.broadcast_message_to_topic(response);
            }
            VoiceAssistantMcpTools::Speak => {
                let args: VoiceAssistantSpeakArgs = serde_json::from_str(&message.0.arguments.to_string()).unwrap_or_default();
                let text = args.text;
                match text.is_empty() {
                    false => {
                        let text = text;
                        let tts_engine = self.tts_engine.clone();
                        let is_speaking = self.is_speaking.clone();
                        let correlation_id = message.0.correlation_id.clone();
                        match tts_engine {
                            Some(tts) => {
                                let response = InvokeToolResponse::success(&correlation_id, &format!("Speaking: {text}"));
                                broadcaster.broadcast_message_to_topic(response);
                                std::thread::spawn(move || {
                                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
                                    match runtime {
                                        Ok(runtime) => {
                                            runtime.block_on(async move {
                                                if let Ok(mut speaking) = is_speaking.lock() {
                                                    *speaking = true;
                                                }
                                                if let Err(error) = tts.speak(&text).await {
                                                    error!("Voice Assistant: speak tool TTS failed: {error}");
                                                }
                                                if let Ok(mut speaking) = is_speaking.lock() {
                                                    *speaking = false;
                                                }
                                            });
                                        }
                                        Err(error) => {
                                            error!("Voice Assistant: failed to create runtime for speak tool: {error}");
                                        }
                                    }
                                });
                            }
                            None => {
                                let response = InvokeToolResponse::error(&correlation_id, "TTS engine not initialized");
                                broadcaster.broadcast_message_to_topic(response);
                            }
                        }
                    }
                    _ => {
                        let response = InvokeToolResponse::error(&message.0.correlation_id, "text must not be empty");
                        broadcaster.broadcast_message_to_topic(response);
                    }
                }
            }
        }
    }
}
