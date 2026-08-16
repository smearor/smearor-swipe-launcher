use crate::mcp::handler::prompt_catalog_resource::PromptCatalogResourceResponse;
use crate::memory::EntityState;
use crate::service::VoiceAssistantService;
use serde::Deserialize;
use serde::Serialize;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::UnknownResourceError;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_voice_assistant_model::EmbeddingsResourceResponse;
use smearor_voice_assistant_model::GgufMetadataResponse;
use smearor_voice_assistant_model::ModelEntryResponse;
use smearor_voice_assistant_model::ModelsResourceResponse;
use smearor_voice_assistant_model::RankingEntry;
use smearor_voice_assistant_model::StatusResourceResponse;
use smearor_voice_assistant_model::SttResourceResponse;
use smearor_voice_assistant_model::ToolCatalogResourceResponse;
use smearor_voice_assistant_model::ToolCatalogResponseEntry;
use smearor_voice_assistant_model::TtsResourceResponse;
use smearor_voice_assistant_model::VoiceAssistantMcpResources;

/// Response for the `memory://entities` resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryEntitiesResourceResponse {
    /// All entity states in the store.
    pub entities: Vec<EntityState>,
}

impl McpResourceHandler<VoiceAssistantMcpResources> for VoiceAssistantService {
    fn get_response(&self, request: &ResourceRequest<VoiceAssistantMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        match request.resource {
            VoiceAssistantMcpResources::Status => {
                let state = self.state.read().map(|state| format!("{:?}", *state)).unwrap_or_else(|_| "Unknown".to_string());
                let response_type = self.current_response_type.read().map(|rt| rt.clone()).unwrap_or(None);
                let make_ranking = |ranking: &[(String, f32)]| -> Vec<RankingEntry> {
                    ranking
                        .iter()
                        .map(|(name, score)| RankingEntry {
                            name: name.clone(),
                            score: *score,
                        })
                        .collect()
                };
                let tool_ranking = self.last_tool_ranking.read().map(|r| make_ranking(&r)).unwrap_or_default();
                let resource_ranking = self.last_resource_ranking.read().map(|r| make_ranking(&r)).unwrap_or_default();
                let prompt_ranking = self.last_prompt_ranking.read().map(|r| make_ranking(&r)).unwrap_or_default();
                let response = StatusResourceResponse {
                    state,
                    transcript: self.current_transcript.read().map(|t| t.clone()).unwrap_or_default(),
                    final_answer: self.current_answer.read().map(|a| a.clone()).unwrap_or_default(),
                    response_type,
                    last_tool_ranking: tool_ranking,
                    last_resource_ranking: resource_ranking,
                    last_prompt_ranking: prompt_ranking,
                };
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
                InvokeResourceResponse::success(correlation_id, &json)
            }
            VoiceAssistantMcpResources::ToolCatalog => {
                let catalog = self.tool_catalog.read().unwrap_or_else(|e| e.into_inner());
                let response = ToolCatalogResourceResponse {
                    tools: catalog
                        .iter()
                        .map(|t| ToolCatalogResponseEntry {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            input_schema: t.input_schema.clone(),
                        })
                        .collect(),
                };
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{\"tools\":[]}".to_string());
                InvokeResourceResponse::success(correlation_id, &json)
            }
            VoiceAssistantMcpResources::Llm => {
                let json = if let Some(backend) = self.llm_backend.read().unwrap().clone() {
                    let tool_calls = self.last_tool_calls.read().map(|c| c.clone()).unwrap_or_default();
                    let report = backend.resource_report(tool_calls);
                    serde_json::to_value(&report).unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize LLM resource report"}))
                } else {
                    serde_json::json!({"error": "LLM backend not initialized"})
                };
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            VoiceAssistantMcpResources::Stt => {
                let response = SttResourceResponse {
                    whisper_model_path: self.config.whisper_model_path.clone(),
                    audio_sample_rate: self.config.audio_sample_rate,
                    audio_channels: self.config.audio_channels,
                    max_recording_seconds: self.config.max_recording_seconds,
                    silence_threshold_seconds: self.config.silence_threshold_seconds,
                    language: self.config.language.clone(),
                };
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
                InvokeResourceResponse::success(correlation_id, &json)
            }
            VoiceAssistantMcpResources::Tts => {
                let tts = &self.config.tts;
                let response = TtsResourceResponse {
                    enabled: tts.enabled,
                    tts_enabled_mcp: tts.tts_enabled_mcp,
                    conversion_step: tts.conversion_step,
                    phonemize_enabled: tts.phonemize_enabled,
                    model_path: tts.model_path.clone(),
                    config_path: tts.config_path.clone(),
                    model_type: format!("{:?}", tts.model_type),
                    model_sample_rate: tts.model_sample_rate,
                    language: tts.phonemizer_config.language.clone(),
                    voice: tts.voice.clone(),
                };
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
                InvokeResourceResponse::success(correlation_id, &json)
            }
            VoiceAssistantMcpResources::Embeddings => {
                let threshold = self.tool_selection_threshold.read().map(|g| *g).unwrap_or(0.0);
                let response = if let Some(engine) = &self.embedding_engine {
                    EmbeddingsResourceResponse {
                        model_name: Some(engine.model_name().to_string()),
                        is_fallback: Some(engine.is_fallback()),
                        configured_model: self.config.embedding_model.clone(),
                        cache_entry_count: Some(engine.cache_entry_count()),
                        tool_selection_threshold: threshold,
                        error: None,
                    }
                } else {
                    EmbeddingsResourceResponse {
                        model_name: None,
                        is_fallback: None,
                        configured_model: self.config.embedding_model.clone(),
                        cache_entry_count: None,
                        tool_selection_threshold: threshold,
                        error: Some("Embedding engine not initialized".to_string()),
                    }
                };
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
                InvokeResourceResponse::success(correlation_id, &json)
            }
            VoiceAssistantMcpResources::MemoryEntities => {
                let store = self.entity_store.read().unwrap_or_else(|e| e.into_inner());
                let response = MemoryEntitiesResourceResponse {
                    entities: store.values().cloned().collect(),
                };
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{\"entities\":[]}".to_string());
                InvokeResourceResponse::success(correlation_id, &json)
            }
            VoiceAssistantMcpResources::Models => {
                let mut models: Vec<ModelEntryResponse> = Vec::new();
                let models_dir = smearor_voice_assistant_model::xdg_models_dir();
                if let Ok(entries) = std::fs::read_dir(&models_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".gguf") {
                                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                                let path = format!("{models_dir}/{name}");

                                let mut metadata: Option<GgufMetadataResponse> = None;
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
                                            metadata = Some(GgufMetadataResponse {
                                                architecture: arch.clone(),
                                                name: get("general.name"),
                                                context_length: get_arch("context_length"),
                                                embedding_length: get_arch("embedding_length"),
                                                block_count: get_arch("block_count"),
                                                head_count: get_arch("attention.head_count"),
                                                head_count_kv: get_arch("attention.head_count_kv"),
                                                file_type: get("general.file_type"),
                                                quantization_version: get("general.quantization_version"),
                                                tokenizer_model: get("tokenizer.ggml.model"),
                                                tensor_count: gguf_file.header.tensor_count,
                                                version: gguf_file.header.version,
                                            });
                                            break;
                                        }
                                    }
                                }

                                models.push(ModelEntryResponse {
                                    filename: name.to_string(),
                                    path,
                                    size_bytes: size,
                                    size_mb: (size as f64) / 1_048_576.0,
                                    metadata,
                                });
                            }
                        }
                    }
                }
                let current_model = if let Some(backend) = self.llm_backend.read().unwrap().clone() {
                    let report = backend.resource_report(Vec::new());
                    report.model_path.unwrap_or_else(|| self.config.llm_model_path.clone())
                } else {
                    self.config.llm_model_path.clone()
                };
                let response = ModelsResourceResponse {
                    current_model,
                    available_models: models,
                };
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
                InvokeResourceResponse::success(correlation_id, &json)
            }
            VoiceAssistantMcpResources::PromptCatalog => {
                let catalog = self.prompt_catalog.read().unwrap_or_else(|e| e.into_inner());
                let response = PromptCatalogResourceResponse::new(catalog.iter().cloned().collect());
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{\"prompts\":[]}".to_string());
                InvokeResourceResponse::success(correlation_id, &json)
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
