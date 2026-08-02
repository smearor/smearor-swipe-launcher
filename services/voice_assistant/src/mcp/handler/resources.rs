use crate::service::VoiceAssistantService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::UnknownResourceError;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_voice_assistant_model::VoiceAssistantMcpResources;

impl McpResourceHandler<VoiceAssistantMcpResources> for VoiceAssistantService {
    fn get_response(&self, request: &ResourceRequest<VoiceAssistantMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        match request.resource {
            VoiceAssistantMcpResources::Status => {
                let state = self.state.read().map(|state| format!("{:?}", *state)).unwrap_or_else(|_| "Unknown".to_string());
                let response_type = self.current_response_type.read().map(|rt| rt.clone()).unwrap_or(None);
                let tool_ranking = self.last_tool_ranking.read().map(|r| r.clone()).unwrap_or_default();
                let tool_ranking_json: Vec<serde_json::Value> = tool_ranking
                    .iter()
                    .map(|(name, score)| serde_json::json!({"name": name, "score": score}))
                    .collect();
                let resource_ranking = self.last_resource_ranking.read().map(|r| r.clone()).unwrap_or_default();
                let resource_ranking_json: Vec<serde_json::Value> = resource_ranking
                    .iter()
                    .map(|(name, score)| serde_json::json!({"name": name, "score": score}))
                    .collect();
                let prompt_ranking = self.last_prompt_ranking.read().map(|r| r.clone()).unwrap_or_default();
                let prompt_ranking_json: Vec<serde_json::Value> = prompt_ranking
                    .iter()
                    .map(|(name, score)| serde_json::json!({"name": name, "score": score}))
                    .collect();
                let json = serde_json::json!({
                    "state": state,
                    "transcript": self.current_transcript.read().map(|t| t.clone()).unwrap_or_default(),
                    "final_answer": self.current_answer.read().map(|a| a.clone()).unwrap_or_default(),
                    "response_type": response_type,
                    "last_tool_ranking": tool_ranking_json,
                    "last_resource_ranking": resource_ranking_json,
                    "last_prompt_ranking": prompt_ranking_json,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            VoiceAssistantMcpResources::ToolCatalog => {
                let catalog = self.tool_catalog.read().unwrap_or_else(|e| e.into_inner());
                let json = serde_json::json!({
                    "tools": catalog.iter().map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "input_schema": t.input_schema,
                        })
                    }).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            VoiceAssistantMcpResources::Llm => {
                let json = if let Some(worker) = self.llm_worker.as_ref() {
                    let cfg = worker.config();
                    let tool_calls = self.last_tool_calls.read().map(|c| c.clone()).unwrap_or_default();
                    serde_json::json!({
                        "model_path": cfg.model_path,
                        "n_ctx": cfg.n_ctx,
                        "n_batch": cfg.n_batch,
                        "max_tokens": cfg.max_tokens,
                        "temperature": cfg.temperature,
                        "top_k": cfg.top_k,
                        "top_p": cfg.top_p,
                        "n_threads": cfg.n_threads,
                        "context_overflow_threshold": cfg.context_overflow_threshold,
                        "n_gpu_layers": cfg.gpu_config.n_gpu_layers,
                        "rolling_window_keep_last": cfg.context_config.rolling_window_keep_last,
                        "context_keep_ratio": cfg.context_config.context_keep_ratio,
                        "min_preserve_tokens": cfg.context_config.min_preserve_tokens,
                        "last_tool_calls": tool_calls,
                    })
                } else {
                    serde_json::json!({"error": "LLM worker not initialized"})
                };
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            VoiceAssistantMcpResources::Stt => {
                let json = serde_json::json!({
                    "whisper_model_path": self.config.whisper_model_path,
                    "audio_sample_rate": self.config.audio_sample_rate,
                    "audio_channels": self.config.audio_channels,
                    "max_recording_seconds": self.config.max_recording_seconds,
                    "silence_threshold_seconds": self.config.silence_threshold_seconds,
                    "language": self.config.language,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            VoiceAssistantMcpResources::Tts => {
                let tts = &self.config.tts;
                let json = serde_json::json!({
                    "enabled": tts.enabled,
                    "tts_enabled_mcp": tts.tts_enabled_mcp,
                    "conversion_step": tts.conversion_step,
                    "phonemize_enabled": tts.phonemize_enabled,
                    "model_path": tts.model_path,
                    "config_path": tts.config_path,
                    "model_type": format!("{:?}", tts.model_type),
                    "model_sample_rate": tts.model_sample_rate,
                    "language": tts.phonemizer_config.language,
                    "voice": tts.voice,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            VoiceAssistantMcpResources::Embeddings => {
                let threshold = self.tool_selection_threshold.read().map(|g| *g).unwrap_or(0.0);
                let json = if let Some(engine) = &self.embedding_engine {
                    serde_json::json!({
                        "model_name": engine.model_name(),
                        "is_fallback": engine.is_fallback(),
                        "configured_model": self.config.embedding_model,
                        "cache_entry_count": engine.cache_entry_count(),
                        "tool_selection_threshold": threshold,
                    })
                } else {
                    serde_json::json!({
                        "error": "Embedding engine not initialized",
                        "configured_model": self.config.embedding_model,
                        "tool_selection_threshold": threshold,
                    })
                };
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            VoiceAssistantMcpResources::MemoryEntities => {
                let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
                let json = serde_json::json!({
                    "entities": store.values().collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            VoiceAssistantMcpResources::Models => {
                let mut models: Vec<serde_json::Value> = Vec::new();
                let models_dir = smearor_voice_assistant_model::xdg_models_dir();
                if let Ok(entries) = std::fs::read_dir(&models_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".gguf") {
                                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                                let path = format!("{models_dir}/{name}");

                                let mut gguf_meta = serde_json::json!({});
                                if let Ok(mut file) = std::fs::File::open(&path) {
                                    use std::io::Read;
                                    let mut buf = Vec::new();
                                    let chunk_size = 4_194_304; // 4MB
                                    let max_header_size = 67_108_864; // 64MB
                                    loop {
                                        let mut chunk = vec![0u8; chunk_size];
                                        let n = file.read(&mut chunk).unwrap_or(0);
                                        if n == 0 || buf.len() >= max_header_size {
                                            break;
                                        }
                                        buf.extend_from_slice(&chunk[..n]);
                                        if let Ok(Some(gguf_file)) = gguf::GGUFFile::read(&buf) {
                                            let metadata_map: std::collections::HashMap<String, String> =
                                                gguf_file.header.metadata.iter().map(|m| (m.key.clone(), format!("{:?}", m.value))).collect();
                                            let arch = metadata_map.get("general.architecture").cloned().unwrap_or_default();
                                            let get = |key: &str| metadata_map.get(key).cloned().unwrap_or_default();
                                            let get_arch = |field: &str| -> String {
                                                if !arch.is_empty() {
                                                    let prefixed = format!("{}.{}", arch, field);
                                                    if let Some(v) = metadata_map.get(&prefixed) {
                                                        return v.clone();
                                                    }
                                                }
                                                get(&format!("llama.{}", field))
                                            };
                                            gguf_meta = serde_json::json!({
                                                "architecture": arch,
                                                "name": get("general.name"),
                                                "context_length": get_arch("context_length"),
                                                "embedding_length": get_arch("embedding_length"),
                                                "block_count": get_arch("block_count"),
                                                "head_count": get_arch("attention.head_count"),
                                                "head_count_kv": get_arch("attention.head_count_kv"),
                                                "file_type": get("general.file_type"),
                                                "quantization_version": get("general.quantization_version"),
                                                "tokenizer_model": get("tokenizer.ggml.model"),
                                                "tensor_count": gguf_file.header.tensor_count,
                                                "version": gguf_file.header.version,
                                            });
                                            break;
                                        }
                                    }
                                }

                                models.push(serde_json::json!({
                                    "filename": name,
                                    "path": path,
                                    "size_bytes": size,
                                    "size_mb": (size as f64) / 1_048_576.0,
                                    "metadata": gguf_meta,
                                }));
                            }
                        }
                    }
                }
                let current_model = if let Some(worker) = self.llm_worker.as_ref() {
                    worker.config().model_path
                } else {
                    self.config.llm_model_path.clone()
                };
                let json = serde_json::json!({
                    "current_model": current_model,
                    "available_models": models,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
        }
    }

    fn on_unknown_resource(&self, _correlation_id: &str, _error: UnknownResourceError) -> Option<InvokeResourceResponse> {
        None
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
