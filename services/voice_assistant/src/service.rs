use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use moka::sync::Cache;

use glib::MainContext;
use llama_cpp_4::model::LlamaChatMessage;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_model_mcp::TOPIC_MCP_PROMPT_RESPONSE;
use smearor_model_mcp::TOPIC_MCP_REGISTER_PROMPT;
use smearor_model_mcp::TOPIC_MCP_REGISTER_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_REGISTER_TOOL;
use smearor_model_mcp::TOPIC_MCP_RESOURCE_RESPONSE;
use smearor_model_mcp::TOPIC_MCP_TOOL_RESPONSE;
use smearor_personalization_model::PersonalizationStatusMessage;
use smearor_personalization_model::TOPIC_STATUS as TOPIC_PERSONALIZATION_STATUS;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use smearor_voice_assistant_model::AssistantState;
use smearor_voice_assistant_model::AssistantStatusMessage;
use smearor_voice_assistant_model::PromptCatalogEntry;
use smearor_voice_assistant_model::ResourceCatalogEntry;
use smearor_voice_assistant_model::ToolCatalogEntry;
use smearor_voice_assistant_model::VoiceCommandAction;
use smearor_voice_assistant_model::VoiceCommandMessage;
use tracing::debug;
use tracing::error;
use tracing::trace;
use tracing::warn;
use whisper_rs::WhisperContext;

use crate::audio::capture_audio;
use crate::audio::compute_rms;
use crate::catalog_router::CatalogRouter;
use crate::catalog_router::SharedCatalogRouter;
use crate::config::VoiceAssistantServiceConfig;
use crate::config::WakeWordModelType;
use crate::embedding_engine::SharedEmbeddingEngine;
use crate::llm::LlmInferenceEngine;
use crate::llm::LlmWorker;
use crate::memory::EntityStore;
use crate::memory::SemanticMemory;
use crate::memory::SharedSemanticMemory;
use crate::react::PendingInvocations;
use crate::react::PendingPromptInvocations;
use crate::react::PendingResourceReads;
use crate::tool_router::SharedToolRouter;
use crate::tool_router::ToolRouter;
use crate::training::TrainingHistory;
use crate::training::TrainingTrace;
use crate::transcriber::load_whisper_context;
use crate::transcriber::transcribe_async;
use crate::tts::TtsEngine;
use crate::tts::try_init_tts;
use crate::vad::SharedSileroVad;
use crate::vad::load_vad_engine;
use crate::vad::trim_silence_async;
use crate::wake_word::SharedAudioHandle;
use crate::wake_word::WakeWordDetectorHandle;
use crate::wake_word::WakeWordEvent;
use crate::wake_word::start_shared_audio;
use crate::wake_word::start_wake_word_detection;
use std::str::FromStr;

pub struct VoiceAssistantService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: VoiceAssistantServiceConfig,
    pub state: Arc<RwLock<AssistantState>>,
    pub tool_catalog: Arc<RwLock<Vec<ToolCatalogEntry>>>,
    pub resource_catalog: Arc<RwLock<Vec<ResourceCatalogEntry>>>,
    pub prompt_catalog: Arc<RwLock<Vec<PromptCatalogEntry>>>,
    pub tool_router: SharedToolRouter,
    pub resource_router: SharedCatalogRouter,
    pub prompt_router: SharedCatalogRouter,
    pub tool_cache: crate::tool_cache::ToolCache,
    pub performance_monitor: crate::performance::PerformanceMonitor,
    pub tools_json_cache: Cache<Vec<String>, String>,
    pub whisper_context: Option<Arc<WhisperContext>>,
    pub vad_engine: Option<SharedSileroVad>,
    pub llm_engine: Option<Arc<LlmInferenceEngine>>,
    pub llm_worker: Option<Arc<LlmWorker>>,
    pub entity_store: EntityStore,
    pub semantic_memory: SharedSemanticMemory,
    pub embedding_engine: Option<SharedEmbeddingEngine>,
    pub conversation_history: Arc<RwLock<Vec<LlamaChatMessage>>>,
    pub pending_invocations: PendingInvocations,
    pub pending_resource_reads: PendingResourceReads,
    pub pending_prompt_invocations: PendingPromptInvocations,
    pub current_transcript: Arc<RwLock<String>>,
    pub current_answer: Arc<RwLock<String>>,
    pub current_response_type: Arc<RwLock<Option<String>>>,
    pub active: Arc<Mutex<bool>>,
    pub tts_engine: Option<Arc<TtsEngine>>,
    pub command_sender: Option<tokio::sync::mpsc::UnboundedSender<VoiceCommandMessage>>,
    pub status_sender: Option<tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>>,
    pub training_mode: Arc<Mutex<bool>>,
    pub active_trace: Arc<Mutex<Option<TrainingTrace>>>,
    pub training_history: TrainingHistory,
    pub tool_selection_threshold: Arc<RwLock<f32>>,
    /// Tool names invoked during the last ReAct loop execution.
    pub last_tool_calls: Arc<RwLock<Vec<String>>>,
    /// Top-5 ranked tools with cosine similarity scores from the last tool selection.
    pub last_tool_ranking: Arc<RwLock<Vec<(String, f32)>>>,
    /// Top-5 ranked resources with cosine similarity scores from the last resource selection.
    pub last_resource_ranking: Arc<RwLock<Vec<(String, f32)>>>,
    /// Top-5 ranked prompts with cosine similarity scores from the last prompt selection.
    pub last_prompt_ranking: Arc<RwLock<Vec<(String, f32)>>>,
    /// Runtime override for the system prompt (set via MCP tool). When set,
    /// this takes precedence over the file-based and default prompts.
    pub runtime_system_prompt: Arc<RwLock<Option<String>>>,
    /// Whether wake word detection mode is currently enabled.
    pub wake_word_enabled: Arc<Mutex<bool>>,
    /// Handle to the running wake word detector thread.
    pub wake_word_detector: Arc<Mutex<Option<WakeWordDetectorHandle>>>,
    /// Handle to the shared audio source (continuous cpal capture).
    pub shared_audio: Arc<Mutex<Option<SharedAudioHandle>>>,
    /// Flag indicating whether TTS is currently speaking (used to suppress wake word detection).
    pub is_speaking: Arc<Mutex<bool>>,
    /// Current wake word model type (shared, can be changed at runtime).
    pub wake_word_model: Arc<Mutex<WakeWordModelType>>,
    /// Current wake word detection threshold (shared, can be changed at runtime).
    pub wake_word_threshold: Arc<Mutex<f32>>,
    /// Latest personalization status (locale, timezone, coordinates).
    pub personalization: Arc<RwLock<Option<PersonalizationStatusMessage>>>,
    /// Previous `speech_detected` value from DoA status, used for edge detection.
    pub previous_speech_detected: Arc<Mutex<bool>>,
    /// Timestamp of the first rising edge of `speech_detected`, used for
    /// `min_speech_duration_ms` false-trigger mitigation.
    pub vad_onset_timestamp: Arc<Mutex<Option<std::time::Instant>>>,
    /// Latest calibrated DoA angle (0–359), attached as context metadata
    /// when VAD-triggered listening mode activates.
    pub doa_angle: Arc<RwLock<u16>>,
    /// Latest mapped compass direction from DoA status.
    pub doa_direction: Arc<RwLock<smearor_doa_model::DoaDirection>>,
    /// Cancellation source ID for the VAD grace period exit timer.
    /// When speech resumes during the grace period, the source is removed
    /// to abort the pending deactivation.
    pub vad_grace_cancel: Arc<Mutex<Option<glib::SourceId>>>,
}

impl VoiceAssistantService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_voice_assistant_model::register_json_converters(core_context);

        let mut service_config: VoiceAssistantServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        service_config.whisper_model_path = shellexpand::tilde(&service_config.whisper_model_path).into_owned();
        service_config.llm_model_path = shellexpand::tilde(&service_config.llm_model_path).into_owned();
        service_config.vad_model_path = shellexpand::tilde(&service_config.vad_model_path).into_owned();
        service_config.memory_db_path = shellexpand::tilde(&service_config.memory_db_path).into_owned();
        service_config.tts.model_path = shellexpand::tilde(&service_config.tts.model_path).into_owned();
        service_config.tts.config_path = shellexpand::tilde(&service_config.tts.config_path).into_owned();

        let tool_router = Arc::new(RwLock::new(ToolRouter::new()));
        let resource_router = Arc::new(RwLock::new(CatalogRouter::new()));
        let prompt_router = Arc::new(RwLock::new(CatalogRouter::new()));
        let tool_cache = crate::tool_cache::ToolCache::new();
        let performance_monitor = crate::performance::PerformanceMonitor::new();
        let entity_store = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let conversation_history: Arc<RwLock<Vec<LlamaChatMessage>>> = Arc::new(RwLock::new(Vec::new()));
        let tools_json_cache = Cache::builder().max_capacity(128).time_to_live(std::time::Duration::from_secs(600)).build();
        let tool_selection_threshold = Arc::new(RwLock::new(service_config.tool_selection_threshold));
        let wake_word_model_init = service_config.wake_word.model.clone();
        let wake_word_threshold_init = service_config.wake_word.threshold;

        let mut service = VoiceAssistantService {
            meta: PluginMeta::try_from(&config)?,
            config: service_config,
            core_context,
            state: Arc::new(RwLock::new(AssistantState::Idle)),
            tool_catalog: Arc::new(RwLock::new(Vec::new())),
            resource_catalog: Arc::new(RwLock::new(Vec::new())),
            prompt_catalog: Arc::new(RwLock::new(Vec::new())),
            tool_router,
            resource_router,
            prompt_router,
            tool_cache,
            performance_monitor,
            tools_json_cache,
            whisper_context: None,
            vad_engine: None,
            llm_engine: None,
            llm_worker: None,
            entity_store,
            semantic_memory: Arc::new(RwLock::new(SemanticMemory::uninit())),
            embedding_engine: None,
            conversation_history,
            pending_invocations: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_resource_reads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_prompt_invocations: Arc::new(Mutex::new(std::collections::HashMap::new())),
            current_transcript: Arc::new(RwLock::new(String::new())),
            current_answer: Arc::new(RwLock::new(String::new())),
            current_response_type: Arc::new(RwLock::new(None)),
            active: Arc::new(Mutex::new(false)),
            tts_engine: None,
            command_sender: None,
            status_sender: None,
            training_mode: Arc::new(Mutex::new(false)),
            active_trace: Arc::new(Mutex::new(None)),
            training_history: crate::training::new_training_history(),
            tool_selection_threshold,
            last_tool_calls: Arc::new(RwLock::new(Vec::new())),
            last_tool_ranking: Arc::new(RwLock::new(Vec::new())),
            last_resource_ranking: Arc::new(RwLock::new(Vec::new())),
            last_prompt_ranking: Arc::new(RwLock::new(Vec::new())),
            runtime_system_prompt: Arc::new(RwLock::new(None)),
            wake_word_enabled: Arc::new(Mutex::new(false)),
            wake_word_detector: Arc::new(Mutex::new(None)),
            shared_audio: Arc::new(Mutex::new(None)),
            is_speaking: Arc::new(Mutex::new(false)),
            wake_word_model: Arc::new(Mutex::new(wake_word_model_init)),
            wake_word_threshold: Arc::new(Mutex::new(wake_word_threshold_init)),
            personalization: Arc::new(RwLock::new(None)),
            previous_speech_detected: Arc::new(Mutex::new(false)),
            vad_onset_timestamp: Arc::new(Mutex::new(None)),
            doa_angle: Arc::new(RwLock::new(0)),
            doa_direction: Arc::new(RwLock::new(smearor_doa_model::DoaDirection::default())),
            vad_grace_cancel: Arc::new(Mutex::new(None)),
        };

        // Ensure models are downloaded before loading.
        crate::model_downloader::ensure_model(&service.config.whisper_model_path, &service.config.whisper_model_repo);
        crate::model_downloader::ensure_model(&service.config.llm_model_path, &service.config.llm_model_repo);
        if service.config.vad_enabled {
            crate::model_downloader::ensure_model(&service.config.vad_model_path, &service.config.vad_model_repo);
        }
        if service.config.tts.enabled {
            crate::model_downloader::ensure_model(&service.config.tts.model_path, &service.config.tts.model_repo);
            crate::model_downloader::ensure_model(&service.config.tts.config_path, &service.config.tts.model_repo);
        }

        // Initialize Whisper context.
        match load_whisper_context(&service.config.whisper_model_path) {
            Ok(context) => {
                debug!("Voice Assistant: Whisper context loaded");
                service.whisper_context = Some(context);
            }
            Err(error) => {
                error!("Voice Assistant: Failed to load Whisper context: {error}");
            }
        }

        // Initialize Silero VAD engine for speech trimming.
        if service.config.vad_enabled {
            match load_vad_engine(&service.config.vad_model_path) {
                Ok(vad) => {
                    debug!("Voice Assistant: Silero VAD engine loaded");
                    service.vad_engine = Some(vad);
                }
                Err(error) => {
                    warn!("Voice Assistant: Failed to load Silero VAD engine: {error}");
                }
            }
        }

        // Initialize LLM engine and persistent worker.
        let llm_config = service.config.to_llm_config();
        match LlmInferenceEngine::load(&llm_config) {
            Ok(engine) => {
                debug!("Voice Assistant: LLM engine loaded");
                let worker = LlmWorker::spawn(engine);
                service.llm_worker = Some(Arc::new(worker));
            }
            Err(error) => {
                error!("Voice Assistant: Failed to load LLM engine: {error}");
            }
        }

        // Initialize semantic memory (L2/L3: entity states + long-term facts).
        match SemanticMemory::new(&service.config.memory_db_path, &service.config.embedding_model) {
            Ok(memory) => {
                debug!("Voice Assistant: Semantic memory initialized");
                // Extract shared embedding engine for tool/resource/prompt selection.
                service.embedding_engine = memory.embedding_engine().cloned();
                // Reconstruct entity store from SQLite history.
                if let Ok(entity_map) = memory.reconstruct_entity_store() {
                    if let Ok(mut store) = service.entity_store.write() {
                        *store = entity_map;
                    }
                }
                if let Ok(mut guard) = service.semantic_memory.write() {
                    *guard = memory;
                }
            }
            Err(error) => {
                error!("Voice Assistant: Failed to initialize semantic memory: {error}");
            }
        }

        // Initialize TTS engine.
        service.tts_engine = try_init_tts(&service.config.tts).map(Arc::new);

        // Set up status broadcast channel.
        let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<AssistantStatusMessage>();
        let service_meta_for_broadcast = service.meta.clone();
        let service_core_context_for_broadcast = service.core_context.clone();
        MainContext::default().spawn_local(async move {
            while let Some(status) = status_receiver.recv().await {
                broadcast_status(&service_meta_for_broadcast, &service_core_context_for_broadcast, status);
            }
        });

        // Spawn the async pipeline handler.
        let (command_sender, mut command_receiver) = tokio::sync::mpsc::unbounded_channel::<VoiceCommandMessage>();
        let service_state = service.state.clone();
        let service_whisper = service.whisper_context.clone();
        let service_vad = service.vad_engine.clone();
        let service_llm = service.llm_engine.clone();
        let service_worker = service.llm_worker.clone();
        let service_entity_store = service.entity_store.clone();
        let service_semantic_memory = service.semantic_memory.clone();
        let service_conversation_history = service.conversation_history.clone();
        let service_tool_router = service.tool_router.clone();
        let service_resource_router = service.resource_router.clone();
        let service_prompt_router = service.prompt_router.clone();
        let service_training_mode = service.training_mode.clone();
        let service_active_trace = service.active_trace.clone();
        let service_training_history = service.training_history.clone();
        let service_config = service.config.clone();
        let service_transcript = service.current_transcript.clone();
        let service_answer = service.current_answer.clone();
        let service_response_type = service.current_response_type.clone();
        let service_active = service.active.clone();
        let service_tts = service.tts_engine.clone();
        let service_pending = service.pending_invocations.clone();
        let service_pending_resources = service.pending_resource_reads.clone();
        let service_pending_prompts = service.pending_prompt_invocations.clone();
        let service_tool_catalog = service.tool_catalog.clone();
        let service_resource_catalog = service.resource_catalog.clone();
        let service_prompt_catalog = service.prompt_catalog.clone();
        let service_core_context = service.core_context.clone();
        let service_meta = service.meta.clone();
        let service_status_sender = status_sender.clone();
        let service_performance_monitor = service.performance_monitor.clone();
        let service_tool_selection_threshold = service.tool_selection_threshold.clone();
        let service_last_tool_calls = service.last_tool_calls.clone();
        let service_last_tool_ranking = service.last_tool_ranking.clone();
        let service_last_resource_ranking = service.last_resource_ranking.clone();
        let service_last_prompt_ranking = service.last_prompt_ranking.clone();
        let service_runtime_system_prompt = service.runtime_system_prompt.clone();
        let service_wake_word_enabled = service.wake_word_enabled.clone();
        let service_wake_word_detector = service.wake_word_detector.clone();
        let service_shared_audio = service.shared_audio.clone();
        let service_is_speaking = service.is_speaking.clone();
        let service_wake_word_model = service.wake_word_model.clone();
        let service_wake_word_threshold = service.wake_word_threshold.clone();
        let service_personalization = service.personalization.clone();
        let service_command_sender = command_sender.clone();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    error!("Voice Assistant: failed to create tokio runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                while let Some(message) = command_receiver.recv().await {
                    match message.action {
                        VoiceCommandAction::Activate => {
                            let mut active = service_active.lock().unwrap_or_else(|e| e.into_inner());
                            if *active {
                                debug!("Voice Assistant: already active, ignoring activate");
                                continue;
                            }
                            *active = true;
                            drop(active);

                            // If wake word shared audio is running, stop it before opening
                            // a second cpal input stream (capture_audio). Two simultaneous
                            // input streams on the same device can hang on Linux.
                            let was_wake_word_active = *service_wake_word_enabled.lock().unwrap_or_else(|e| e.into_inner());
                            if was_wake_word_active {
                                debug!("Voice Assistant: stopping shared audio before pipeline");
                                if let Ok(mut guard) = service_wake_word_detector.lock() {
                                    if let Some(mut handle) = guard.take() {
                                        handle.stop();
                                    }
                                }
                                if let Ok(mut guard) = service_shared_audio.lock() {
                                    if let Some(mut handle) = guard.take() {
                                        handle.stop();
                                    }
                                }
                            }

                            // Run the pipeline.
                            Self::run_pipeline_inner(
                                &service_config,
                                &service_state,
                                &service_whisper,
                                &service_vad,
                                &service_llm,
                                &service_worker,
                                &service_entity_store,
                                &service_semantic_memory,
                                &service_conversation_history,
                                &service_tool_router,
                                &service_resource_router,
                                &service_prompt_router,
                                &service_training_mode,
                                &service_active_trace,
                                &service_training_history,
                                &service_transcript,
                                &service_answer,
                                &service_response_type,
                                &service_active,
                                &service_tts,
                                &service_pending,
                                &service_pending_resources,
                                &service_pending_prompts,
                                &service_tool_catalog,
                                &service_resource_catalog,
                                &service_prompt_catalog,
                                &service_core_context,
                                &service_meta,
                                &service_status_sender,
                                &service_performance_monitor,
                                &service_tool_selection_threshold,
                                &service_last_tool_calls,
                                &service_last_tool_ranking,
                                &service_last_resource_ranking,
                                &service_last_prompt_ranking,
                                &service_runtime_system_prompt,
                                &service_is_speaking,
                                &service_personalization,
                            )
                            .await;

                            // If wake word was active before pipeline, restart shared audio
                            // and detector, then return to Standby.
                            if was_wake_word_active {
                                debug!("Voice Assistant: restarting shared audio after pipeline");
                                let audio_rate = service_config.audio_sample_rate;
                                let audio_channels = service_config.audio_channels;
                                match start_shared_audio(audio_rate, audio_channels, 32000, 2) {
                                    Ok(handle) => {
                                        let consumer = handle.get_consumer(0);
                                        if let Some(ww_consumer) = consumer {
                                            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<WakeWordEvent>();
                                            match start_wake_word_detection(
                                                ww_consumer,
                                                service_wake_word_model.lock().unwrap_or_else(|e| e.into_inner()).clone(),
                                                service_config.wake_word.model_path.clone(),
                                                *service_wake_word_threshold.lock().unwrap_or_else(|e| e.into_inner()),
                                                service_is_speaking.clone(),
                                                event_tx,
                                            ) {
                                                Ok(detector_handle) => {
                                                    if let Ok(mut guard) = service_wake_word_detector.lock() {
                                                        *guard = Some(detector_handle);
                                                    }
                                                    if let Ok(mut guard) = service_shared_audio.lock() {
                                                        *guard = Some(handle);
                                                    }
                                                    let ww_command_sender = service_command_sender.clone();
                                                    let ww_active = service_active.clone();
                                                    let ww_enabled = service_wake_word_enabled.clone();
                                                    tokio::spawn(async move {
                                                        while let Some(event) = event_rx.recv().await {
                                                            debug!("Wake word detected (p={:.3}), activating pipeline", event.probability);
                                                            let active = ww_active.lock().unwrap_or_else(|e| e.into_inner());
                                                            if *active {
                                                                debug!("Wake word: already active, ignoring");
                                                                continue;
                                                            }
                                                            let enabled = ww_enabled.lock().unwrap_or_else(|e| e.into_inner());
                                                            if !*enabled {
                                                                break;
                                                            }
                                                            drop(active);
                                                            let _ = ww_command_sender.send(VoiceCommandMessage::activate());
                                                        }
                                                    });
                                                }
                                                Err(error) => {
                                                    error!("Voice Assistant: failed to restart wake word detection: {error}");
                                                }
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        error!("Voice Assistant: failed to restart shared audio: {error}");
                                    }
                                }

                                Self::set_state(&service_state, AssistantState::Standby, &service_status_sender, &service_transcript, &service_answer).await;
                            }
                        }
                        VoiceCommandAction::Deactivate => {
                            let mut active = service_active.lock().unwrap_or_else(|e| e.into_inner());
                            *active = false;
                            debug!("Voice Assistant: deactivated");
                        }
                        VoiceCommandAction::SubmitText => {
                            // Submit text bypasses STT, goes directly to ReAct loop.
                            let mut active = service_active.lock().unwrap_or_else(|e| e.into_inner());
                            if *active {
                                debug!("Voice Assistant: already active, ignoring submit_text");
                                continue;
                            }
                            *active = true;
                            drop(active);

                            Self::run_text_pipeline(
                                &message.text,
                                &service_config,
                                &service_state,
                                &service_llm,
                                &service_worker,
                                &service_entity_store,
                                &service_semantic_memory,
                                &service_conversation_history,
                                &service_tool_router,
                                &service_resource_router,
                                &service_prompt_router,
                                &service_training_mode,
                                &service_active_trace,
                                &service_training_history,
                                &service_transcript,
                                &service_answer,
                                &service_response_type,
                                &service_active,
                                &service_tts,
                                &service_pending,
                                &service_pending_resources,
                                &service_pending_prompts,
                                &service_tool_catalog,
                                &service_resource_catalog,
                                &service_prompt_catalog,
                                &service_core_context,
                                &service_meta,
                                &service_status_sender,
                                &service_tool_selection_threshold,
                                &service_last_tool_calls,
                                &service_last_tool_ranking,
                                &service_last_resource_ranking,
                                &service_last_prompt_ranking,
                                &service_runtime_system_prompt,
                                &service_is_speaking,
                                &service_personalization,
                            )
                            .await;

                            // If wake word is enabled, return to Standby instead of Idle.
                            // (SubmitText does not open a cpal input stream, so no restart needed.)
                            let ww_enabled = *service_wake_word_enabled.lock().unwrap_or_else(|e| e.into_inner());
                            if ww_enabled {
                                Self::set_state(&service_state, AssistantState::Standby, &service_status_sender, &service_transcript, &service_answer).await;
                            }
                        }
                        VoiceCommandAction::ClearConversation => {
                            debug!("Voice Assistant: clearing conversation history");
                            if let Ok(mut history) = service_conversation_history.write() {
                                history.clear();
                            }
                            if let Some(worker) = service_worker.as_ref() {
                                if let Err(error) = worker.clear_conversation().await {
                                    warn!("Voice Assistant: failed to clear LLM KV cache: {error}");
                                }
                            }
                            if let Some(tts) = service_tts.as_ref() {
                                tts.cancel();
                            }
                            if let Ok(mut speaking) = service_is_speaking.lock() {
                                *speaking = false;
                            }
                            if let Ok(mut active) = service_active.lock() {
                                *active = false;
                            }
                            if let Ok(mut transcript) = service_transcript.write() {
                                transcript.clear();
                            }
                            if let Ok(mut answer) = service_answer.write() {
                                answer.clear();
                            }
                            if let Ok(mut response_type) = service_response_type.write() {
                                *response_type = None;
                            }
                            service_performance_monitor.reset();
                            Self::set_state(&service_state, AssistantState::Idle, &service_status_sender, &service_transcript, &service_answer).await;
                        }
                        VoiceCommandAction::EnableWakeWord => {
                            let mut enabled = service_wake_word_enabled.lock().unwrap_or_else(|e| e.into_inner());
                            if *enabled {
                                trace!("Voice Assistant: wake word already enabled, ignoring");
                                continue;
                            }
                            *enabled = true;
                            drop(enabled);

                            trace!("Voice Assistant: enabling wake word detection");

                            // Start shared audio source with 2 consumers (wake word + pipeline).
                            let audio_rate = service_config.audio_sample_rate;
                            let audio_channels = service_config.audio_channels;
                            match start_shared_audio(audio_rate, audio_channels, 32000, 2) {
                                Ok(handle) => {
                                    let consumer = handle.get_consumer(0);

                                    if let Some(ww_consumer) = consumer {
                                        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<WakeWordEvent>();
                                        match start_wake_word_detection(
                                            ww_consumer,
                                            service_wake_word_model.lock().unwrap_or_else(|e| e.into_inner()).clone(),
                                            service_config.wake_word.model_path.clone(),
                                            *service_wake_word_threshold.lock().unwrap_or_else(|e| e.into_inner()),
                                            service_is_speaking.clone(),
                                            event_tx,
                                        ) {
                                            Ok(detector_handle) => {
                                                if let Ok(mut guard) = service_wake_word_detector.lock() {
                                                    *guard = Some(detector_handle);
                                                }
                                                if let Ok(mut guard) = service_shared_audio.lock() {
                                                    *guard = Some(handle);
                                                }

                                                // Set state to Standby.
                                                Self::set_state(
                                                    &service_state,
                                                    AssistantState::Standby,
                                                    &service_status_sender,
                                                    &service_transcript,
                                                    &service_answer,
                                                )
                                                .await;

                                                // Spawn a task to listen for wake word events.
                                                let ww_command_sender = service_command_sender.clone();
                                                let ww_active = service_active.clone();
                                                let ww_enabled = service_wake_word_enabled.clone();
                                                tokio::spawn(async move {
                                                    while let Some(event) = event_rx.recv().await {
                                                        debug!("Wake word detected (p={:.3}), activating pipeline", event.probability);
                                                        let active = ww_active.lock().unwrap_or_else(|e| e.into_inner());
                                                        if *active {
                                                            debug!("Wake word: already active, ignoring");
                                                            continue;
                                                        }
                                                        let enabled = ww_enabled.lock().unwrap_or_else(|e| e.into_inner());
                                                        if !*enabled {
                                                            break;
                                                        }
                                                        drop(active);
                                                        // Send Activate command to trigger the pipeline.
                                                        let _ = ww_command_sender.send(VoiceCommandMessage::activate());
                                                    }
                                                });
                                            }
                                            Err(error) => {
                                                error!("Voice Assistant: failed to start wake word detection: {error}");
                                                *service_wake_word_enabled.lock().unwrap_or_else(|e| e.into_inner()) = false;
                                            }
                                        }
                                    }
                                }
                                Err(error) => {
                                    error!("Voice Assistant: failed to start shared audio: {error}");
                                    *service_wake_word_enabled.lock().unwrap_or_else(|e| e.into_inner()) = false;
                                }
                            }
                        }
                        VoiceCommandAction::DisableWakeWord => {
                            debug!("Voice Assistant: disabling wake word detection");
                            {
                                let mut enabled = service_wake_word_enabled.lock().unwrap_or_else(|e| e.into_inner());
                                *enabled = false;
                            }
                            // Stop the wake word detector thread.
                            if let Ok(mut guard) = service_wake_word_detector.lock() {
                                if let Some(mut handle) = guard.take() {
                                    handle.stop();
                                }
                            }
                            // Stop the shared audio source.
                            if let Ok(mut guard) = service_shared_audio.lock() {
                                if let Some(mut handle) = guard.take() {
                                    handle.stop();
                                }
                            }
                            // Transition from Standby to Idle.
                            if let Ok(state_guard) = service_state.read() {
                                if *state_guard == AssistantState::Standby {
                                    drop(state_guard);
                                    Self::set_state(&service_state, AssistantState::Idle, &service_status_sender, &service_transcript, &service_answer).await;
                                }
                            }
                        }
                        VoiceCommandAction::SetWakeWordModel => {
                            // Parse JSON from text field: {"model":"Alexa","threshold":0.1}
                            let new_model: Option<WakeWordModelType>;
                            let new_threshold: Option<f32>;
                            match serde_json::from_str::<serde_json::Value>(&message.text) {
                                Ok(json) => {
                                    if let Some(model_str) = json.get("model").and_then(|v| v.as_str()) {
                                        match WakeWordModelType::from_str(model_str) {
                                            Ok(model) => new_model = Some(model),
                                            Err(error) => {
                                                error!("Voice Assistant: invalid wake word model '{model_str}': {error}");
                                                new_model = None;
                                            }
                                        }
                                    } else {
                                        new_model = None;
                                    }
                                    if let Some(threshold_val) = json.get("threshold").and_then(|v| v.as_f64()) {
                                        new_threshold = Some(threshold_val as f32);
                                    } else {
                                        new_threshold = None;
                                    }
                                }
                                Err(error) => {
                                    error!("Voice Assistant: failed to parse SetWakeWordModel JSON: {error}");
                                    new_model = None;
                                    new_threshold = None;
                                }
                            }

                            // Update shared state.
                            let model_changed = if let Some(model) = &new_model {
                                let mut guard = service_wake_word_model.lock().unwrap_or_else(|e| e.into_inner());
                                let changed = *guard != *model;
                                *guard = model.clone();
                                changed
                            } else {
                                false
                            };
                            if let Some(threshold) = new_threshold {
                                *service_wake_word_threshold.lock().unwrap_or_else(|e| e.into_inner()) = threshold;
                            }

                            // If wake word is currently enabled and model changed, restart detector.
                            let was_enabled = *service_wake_word_enabled.lock().unwrap_or_else(|e| e.into_inner());
                            if was_enabled && (model_changed || new_threshold.is_some()) {
                                debug!("Voice Assistant: restarting wake word detector with new model/threshold");
                                // Stop existing detector.
                                if let Ok(mut guard) = service_wake_word_detector.lock() {
                                    if let Some(mut handle) = guard.take() {
                                        handle.stop();
                                    }
                                }
                                // Stop existing shared audio.
                                if let Ok(mut guard) = service_shared_audio.lock() {
                                    if let Some(mut handle) = guard.take() {
                                        handle.stop();
                                    }
                                }
                                // Reset enabled flag so EnableWakeWord re-starts everything.
                                *service_wake_word_enabled.lock().unwrap_or_else(|e| e.into_inner()) = false;
                                // Re-send EnableWakeWord to restart with new config.
                                let _ = service_command_sender.send(VoiceCommandMessage::enable_wake_word());
                            }
                        }
                    }
                }
            });
        });

        // Store the command sender for later use.
        service.command_sender = Some(command_sender);
        service.status_sender = Some(status_sender);

        // Auto-enable wake word detection if configured.
        if service.config.wake_word.auto_enable {
            trace!("Voice Assistant: auto-enabling wake word detection");
            service.enable_wake_word();
        }

        service.register_mcp_capabilities();
        Ok(service)
    }

    /// Sets the assistant state and broadcasts a status update.
    async fn set_state(
        state: &Arc<RwLock<AssistantState>>,
        new_state: AssistantState,
        status_sender: &tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
        _transcript: &Arc<RwLock<String>>,
        _answer: &Arc<RwLock<String>>,
    ) {
        if let Ok(mut state_guard) = state.write() {
            *state_guard = new_state.clone();
        }
        let status = AssistantStatusMessage::new(new_state);
        let _ = status_sender.send(status);
    }

    /// Runs the complete voice pipeline: capture -> STT -> ReAct -> status.
    async fn run_pipeline_inner(
        config: &VoiceAssistantServiceConfig,
        state: &Arc<RwLock<AssistantState>>,
        whisper_context: &Option<Arc<WhisperContext>>,
        vad_engine: &Option<SharedSileroVad>,
        llm_engine: &Option<Arc<LlmInferenceEngine>>,
        llm_worker: &Option<Arc<LlmWorker>>,
        entity_store: &EntityStore,
        semantic_memory: &SharedSemanticMemory,
        conversation_history: &Arc<RwLock<Vec<LlamaChatMessage>>>,
        tool_router: &SharedToolRouter,
        resource_router: &SharedCatalogRouter,
        prompt_router: &SharedCatalogRouter,
        training_mode: &Arc<Mutex<bool>>,
        active_trace: &Arc<Mutex<Option<TrainingTrace>>>,
        training_history: &TrainingHistory,
        transcript: &Arc<RwLock<String>>,
        answer: &Arc<RwLock<String>>,
        response_type: &Arc<RwLock<Option<String>>>,
        active: &Arc<Mutex<bool>>,
        tts_engine: &Option<Arc<TtsEngine>>,
        pending_invocations: &PendingInvocations,
        pending_resource_reads: &PendingResourceReads,
        pending_prompt_invocations: &PendingPromptInvocations,
        tool_catalog: &Arc<RwLock<Vec<ToolCatalogEntry>>>,
        resource_catalog: &Arc<RwLock<Vec<ResourceCatalogEntry>>>,
        prompt_catalog: &Arc<RwLock<Vec<PromptCatalogEntry>>>,
        core_context: &Option<FfiCoreContext>,
        meta: &PluginMeta,
        status_sender: &tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
        performance_monitor: &crate::performance::PerformanceMonitor,
        tool_selection_threshold: &Arc<RwLock<f32>>,
        last_tool_calls: &Arc<RwLock<Vec<String>>>,
        last_tool_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        last_resource_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        last_prompt_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        runtime_system_prompt: &Arc<RwLock<Option<String>>>,
        is_speaking: &Arc<Mutex<bool>>,
        personalization: &Arc<RwLock<Option<PersonalizationStatusMessage>>>,
    ) {
        // 1. Capture audio.
        Self::set_state(state, AssistantState::Listening, status_sender, transcript, answer).await;

        let (_stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let samples = match capture_audio(config, stop_rx).await {
            Ok(samples) => samples,
            Err(error) => {
                error!("Voice Assistant: Audio capture failed: {error}");
                Self::set_error(state, &error.to_string(), answer).await;
                let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
                *active_guard = false;
                return;
            }
        };

        // 2. Check audio energy before transcription.
        // Whisper is known to hallucinate text from near-silence audio,
        // producing phrases like "Vielen Dank." from background noise.
        // Skip transcription entirely if the RMS energy is too low.
        const MIN_AUDIO_RMS: f32 = 0.01;
        let rms = compute_rms(&samples);
        debug!("Voice Assistant: Audio RMS energy: {rms:.6} ({} samples)", samples.len());
        if rms < MIN_AUDIO_RMS {
            debug!("Voice Assistant: Audio energy below threshold ({rms:.6} < {MIN_AUDIO_RMS}), skipping transcription (likely silence)");
            Self::set_state(state, AssistantState::Idle, status_sender, transcript, answer).await;
            let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
            *active_guard = false;
            return;
        }

        // 2b. VAD trim: remove leading/trailing non-speech segments.
        // Silero VAD classifies 512-sample (32ms) frames as speech/non-speech
        // and trims the buffer to only the speech segment. This eliminates
        // background noise that causes Whisper hallucinations.
        let samples = if let Some(vad) = vad_engine {
            let original_len = samples.len();
            match trim_silence_async(vad.clone(), samples.clone(), config.vad_threshold).await {
                Ok(trimmed) if trimmed.is_empty() => {
                    debug!("Voice Assistant: VAD detected no speech, skipping transcription");
                    Self::set_state(state, AssistantState::Idle, status_sender, transcript, answer).await;
                    let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
                    *active_guard = false;
                    return;
                }
                Ok(trimmed) => {
                    debug!("Voice Assistant: VAD trimmed {original_len} -> {} samples", trimmed.len());
                    trimmed
                }
                Err(error) => {
                    warn!("Voice Assistant: VAD failed: {error}, using original audio");
                    samples
                }
            }
        } else {
            samples
        };

        // 3. Transcribe.
        Self::set_state(state, AssistantState::ProcessingStt, status_sender, transcript, answer).await;

        let whisper_ctx = match whisper_context {
            Some(ctx) => ctx.clone(),
            None => {
                error!("Voice Assistant: Whisper context not initialized");
                Self::set_error(state, "Whisper context not initialized", answer).await;
                let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
                *active_guard = false;
                return;
            }
        };

        let stt_start = std::time::Instant::now();
        let transcribed = match transcribe_async(whisper_ctx, samples, config.language.clone()).await {
            Ok(text) => {
                performance_monitor.record_speech_recognition(stt_start.elapsed());
                text
            }
            Err(error) => {
                error!("Voice Assistant: STT failed: {error}");
                Self::set_error(state, &error.to_string(), answer).await;
                let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
                *active_guard = false;
                return;
            }
        };

        debug!("Voice Assistant: Transcribed: {}", transcribed);
        if let Ok(mut transcript_guard) = transcript.write() {
            *transcript_guard = transcribed.clone();
        }

        // Skip ReAct loop if transcription is empty or whitespace-only.
        if transcribed.trim().is_empty() {
            debug!("Voice Assistant: Empty transcription, skipping ReAct loop");
            Self::set_state(state, AssistantState::Idle, status_sender, transcript, answer).await;
            let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
            *active_guard = false;
            return;
        }

        // 4. ReAct loop.
        Self::run_react(
            &transcribed,
            config,
            state,
            llm_engine,
            llm_worker,
            entity_store,
            semantic_memory,
            conversation_history,
            tool_router,
            resource_router,
            prompt_router,
            training_mode,
            active_trace,
            training_history,
            transcript,
            answer,
            response_type,
            active,
            tts_engine,
            false,
            is_speaking,
            pending_invocations,
            pending_resource_reads,
            pending_prompt_invocations,
            tool_catalog,
            resource_catalog,
            prompt_catalog,
            core_context,
            meta,
            status_sender,
            tool_selection_threshold,
            last_tool_calls,
            last_tool_ranking,
            last_resource_ranking,
            last_prompt_ranking,
            runtime_system_prompt,
            personalization,
        )
        .await;
    }

    /// Runs the text pipeline (bypasses STT).
    async fn run_text_pipeline(
        text: &str,
        config: &VoiceAssistantServiceConfig,
        state: &Arc<RwLock<AssistantState>>,
        llm_engine: &Option<Arc<LlmInferenceEngine>>,
        llm_worker: &Option<Arc<LlmWorker>>,
        entity_store: &EntityStore,
        semantic_memory: &SharedSemanticMemory,
        conversation_history: &Arc<RwLock<Vec<LlamaChatMessage>>>,
        tool_router: &SharedToolRouter,
        resource_router: &SharedCatalogRouter,
        prompt_router: &SharedCatalogRouter,
        training_mode: &Arc<Mutex<bool>>,
        active_trace: &Arc<Mutex<Option<TrainingTrace>>>,
        training_history: &TrainingHistory,
        transcript: &Arc<RwLock<String>>,
        answer: &Arc<RwLock<String>>,
        response_type: &Arc<RwLock<Option<String>>>,
        active: &Arc<Mutex<bool>>,
        tts_engine: &Option<Arc<TtsEngine>>,
        pending_invocations: &PendingInvocations,
        pending_resource_reads: &PendingResourceReads,
        pending_prompt_invocations: &PendingPromptInvocations,
        tool_catalog: &Arc<RwLock<Vec<ToolCatalogEntry>>>,
        resource_catalog: &Arc<RwLock<Vec<ResourceCatalogEntry>>>,
        prompt_catalog: &Arc<RwLock<Vec<PromptCatalogEntry>>>,
        core_context: &Option<FfiCoreContext>,
        meta: &PluginMeta,
        status_sender: &tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
        tool_selection_threshold: &Arc<RwLock<f32>>,
        last_tool_calls: &Arc<RwLock<Vec<String>>>,
        last_tool_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        last_resource_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        last_prompt_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        runtime_system_prompt: &Arc<RwLock<Option<String>>>,
        is_speaking: &Arc<Mutex<bool>>,
        personalization: &Arc<RwLock<Option<PersonalizationStatusMessage>>>,
    ) {
        debug!("Voice Assistant: Text pipeline: {}", text);
        if let Ok(mut transcript_guard) = transcript.write() {
            *transcript_guard = text.to_string();
        }

        Self::run_react(
            text,
            config,
            state,
            llm_engine,
            llm_worker,
            entity_store,
            semantic_memory,
            conversation_history,
            tool_router,
            resource_router,
            prompt_router,
            training_mode,
            active_trace,
            training_history,
            transcript,
            answer,
            response_type,
            active,
            tts_engine,
            true,
            is_speaking,
            pending_invocations,
            pending_resource_reads,
            pending_prompt_invocations,
            tool_catalog,
            resource_catalog,
            prompt_catalog,
            core_context,
            meta,
            status_sender,
            tool_selection_threshold,
            last_tool_calls,
            last_tool_ranking,
            last_resource_ranking,
            last_prompt_ranking,
            runtime_system_prompt,
            personalization,
        )
        .await;
    }

    /// Runs the ReAct loop and handles the result.
    async fn run_react(
        user_text: &str,
        config: &VoiceAssistantServiceConfig,
        state: &Arc<RwLock<AssistantState>>,
        llm_engine: &Option<Arc<LlmInferenceEngine>>,
        llm_worker: &Option<Arc<LlmWorker>>,
        entity_store: &EntityStore,
        semantic_memory: &SharedSemanticMemory,
        conversation_history: &Arc<RwLock<Vec<LlamaChatMessage>>>,
        tool_router: &SharedToolRouter,
        resource_router: &SharedCatalogRouter,
        prompt_router: &SharedCatalogRouter,
        training_mode: &Arc<Mutex<bool>>,
        active_trace: &Arc<Mutex<Option<TrainingTrace>>>,
        training_history: &TrainingHistory,
        transcript: &Arc<RwLock<String>>,
        answer: &Arc<RwLock<String>>,
        response_type: &Arc<RwLock<Option<String>>>,
        active: &Arc<Mutex<bool>>,
        tts_engine: &Option<Arc<TtsEngine>>,
        is_mcp: bool,
        is_speaking: &Arc<Mutex<bool>>,
        pending_invocations: &PendingInvocations,
        pending_resource_reads: &PendingResourceReads,
        pending_prompt_invocations: &PendingPromptInvocations,
        tool_catalog: &Arc<RwLock<Vec<ToolCatalogEntry>>>,
        resource_catalog: &Arc<RwLock<Vec<ResourceCatalogEntry>>>,
        prompt_catalog: &Arc<RwLock<Vec<PromptCatalogEntry>>>,
        core_context: &Option<FfiCoreContext>,
        meta: &PluginMeta,
        status_sender: &tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
        tool_selection_threshold: &Arc<RwLock<f32>>,
        last_tool_calls: &Arc<RwLock<Vec<String>>>,
        last_tool_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        last_resource_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        last_prompt_ranking: &Arc<RwLock<Vec<(String, f32)>>>,
        runtime_system_prompt: &Arc<RwLock<Option<String>>>,
        personalization: &Arc<RwLock<Option<PersonalizationStatusMessage>>>,
    ) {
        // Create a temporary service reference for the ReAct loop.
        let temp_service = VoiceAssistantService {
            meta: meta.clone(),
            core_context: core_context.clone(),
            config: config.clone(),
            state: state.clone(),
            tool_catalog: tool_catalog.clone(),
            resource_catalog: resource_catalog.clone(),
            prompt_catalog: prompt_catalog.clone(),
            tool_router: tool_router.clone(),
            resource_router: resource_router.clone(),
            prompt_router: prompt_router.clone(),
            tool_cache: crate::tool_cache::ToolCache::new(),
            performance_monitor: crate::performance::PerformanceMonitor::new(),
            tools_json_cache: Cache::builder().build(),
            whisper_context: None,
            vad_engine: None,
            llm_engine: llm_engine.clone(),
            llm_worker: llm_worker.clone(),
            entity_store: entity_store.clone(),
            semantic_memory: semantic_memory.clone(),
            embedding_engine: semantic_memory.read().ok().and_then(|m| m.embedding_engine().cloned()),
            conversation_history: conversation_history.clone(),
            pending_invocations: pending_invocations.clone(),
            pending_resource_reads: pending_resource_reads.clone(),
            pending_prompt_invocations: pending_prompt_invocations.clone(),
            current_transcript: transcript.clone(),
            current_answer: answer.clone(),
            current_response_type: response_type.clone(),
            active: active.clone(),
            tts_engine: None,
            command_sender: None,
            status_sender: None,
            training_mode: training_mode.clone(),
            active_trace: active_trace.clone(),
            training_history: training_history.clone(),
            tool_selection_threshold: tool_selection_threshold.clone(),
            last_tool_calls: last_tool_calls.clone(),
            last_tool_ranking: last_tool_ranking.clone(),
            last_resource_ranking: last_resource_ranking.clone(),
            last_prompt_ranking: last_prompt_ranking.clone(),
            runtime_system_prompt: runtime_system_prompt.clone(),
            wake_word_enabled: Arc::new(Mutex::new(false)),
            wake_word_detector: Arc::new(Mutex::new(None)),
            shared_audio: Arc::new(Mutex::new(None)),
            is_speaking: Arc::new(Mutex::new(false)),
            wake_word_model: Arc::new(Mutex::new(WakeWordModelType::default())),
            wake_word_threshold: Arc::new(Mutex::new(0.1)),
            personalization: personalization.clone(),
            previous_speech_detected: Arc::new(Mutex::new(false)),
            vad_onset_timestamp: Arc::new(Mutex::new(None)),
            doa_angle: Arc::new(RwLock::new(0)),
            doa_direction: Arc::new(RwLock::new(smearor_doa_model::DoaDirection::default())),
            vad_grace_cancel: Arc::new(Mutex::new(None)),
        };

        Self::set_state(state, AssistantState::ThinkingLlm, status_sender, transcript, answer).await;

        // Proactively trim KV cache when conversation history is long,
        // keeping recent context and avoiding overflow during generation.
        if let Some(worker) = llm_worker.as_ref() {
            let history_len = conversation_history.read().map(|h| h.len()).unwrap_or(0);
            let max_messages = config.max_history_messages;
            if history_len >= max_messages {
                let n_ctx = worker.config().n_ctx as usize;
                let target_tokens = (n_ctx as f64 * config.context_keep_ratio) as usize;
                debug!(
                    "Voice Assistant: proactively trimming KV cache to ~{} tokens (history: {}/{})",
                    target_tokens, history_len, max_messages
                );
                if let Err(error) = worker.trim_context(target_tokens).await {
                    warn!("Voice Assistant: proactive trim_context failed: {error}");
                }
            }
        }

        match temp_service.execute_react_loop(user_text).await {
            Ok(final_answer) => {
                debug!("Voice Assistant: Final answer: {}", final_answer);
                if let Ok(mut answer_guard) = answer.write() {
                    *answer_guard = final_answer.clone();
                }
                if let Ok(mut rt_guard) = response_type.write() {
                    *rt_guard = Some("final_answer".to_string());
                }

                // Speak the final answer via TTS if engine is available.
                // For MCP text input, check tts_enabled_mcp flag.
                let tts_allowed = !is_mcp || config.tts.tts_enabled_mcp;
                if let Some(tts) = tts_engine.as_ref() {
                    if tts_allowed {
                        Self::set_state(state, AssistantState::Speaking, status_sender, transcript, answer).await;
                        if let Ok(mut speaking) = is_speaking.lock() {
                            *speaking = true;
                        }
                        if let Err(error) = tts.speak(&final_answer).await {
                            warn!("Voice Assistant: TTS playback failed: {error}");
                        }
                        if let Ok(mut speaking) = is_speaking.lock() {
                            *speaking = false;
                        }
                    }
                }

                Self::set_state(state, AssistantState::Idle, status_sender, transcript, answer).await;
            }
            Err(error) => {
                error!("Voice Assistant: ReAct loop failed: {error}");
                Self::set_state(state, AssistantState::Error, status_sender, transcript, answer).await;
                if let Ok(mut answer_guard) = answer.write() {
                    *answer_guard = match error {
                        crate::react::AssistantError::MaxIterationsReached => {
                            "Ich konnte die Anfrage nicht in der vorgegebenen Zeit verarbeiten. Bitte versuche es erneut.".to_string()
                        }
                        _ => error.to_string(),
                    };
                }
                debug!("Voice Assistant: Error state: {}", error);
                Self::set_state(state, AssistantState::Idle, status_sender, transcript, answer).await;
            }
        }
        temp_service.performance_monitor.log_summary();

        let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
        *active_guard = false;
    }

    /// Sets the error state and broadcasts a status update.
    async fn set_error(state: &Arc<RwLock<AssistantState>>, error_message: &str, answer: &Arc<RwLock<String>>) {
        if let Ok(mut state_guard) = state.write() {
            *state_guard = AssistantState::Error;
        }
        if let Ok(mut answer_guard) = answer.write() {
            *answer_guard = error_message.to_string();
        }
        debug!("Voice Assistant: Error state: {}", error_message);
    }

    /// Activates the voice assistant pipeline.
    pub fn activate(&self) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(VoiceCommandMessage::activate());
        }
    }

    /// Deactivates the voice assistant pipeline.
    pub fn deactivate(&self) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(VoiceCommandMessage::deactivate());
        }
    }

    /// Submits a text command directly (bypassing STT).
    pub fn submit_text(&self, text: &str) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(VoiceCommandMessage::submit_text(text));
        }
    }

    /// Clears conversation history and LLM KV cache for a fresh session.
    pub fn clear_conversation(&self) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(VoiceCommandMessage::clear_conversation());
        }
    }

    /// Enables wake word detection mode (continuous listening for wake word).
    pub fn enable_wake_word(&self) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(VoiceCommandMessage::enable_wake_word());
        }
    }

    /// Disables wake word detection mode and returns to idle.
    pub fn disable_wake_word(&self) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(VoiceCommandMessage::disable_wake_word());
        }
    }

    /// Changes the wake word model and/or threshold at runtime.
    /// If wake word detection is currently active, the detector is restarted with the new settings.
    pub fn set_wake_word_model(&self, model: &str, threshold: Option<f32>) {
        let json = match (model, threshold) {
            (m, Some(t)) => format!(r#"{{"model":"{m}","threshold":{t}}}"#),
            (m, None) => format!(r#"{{"model":"{m}"}}"#),
        };
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(VoiceCommandMessage::set_wake_word_model(&json));
        }
    }

    /// Returns a snapshot of current performance metrics.
    pub fn performance_report(&self) -> crate::performance::PerformanceReport {
        self.performance_monitor.snapshot()
    }
}

impl MessageBroadcaster for VoiceAssistantService {}

impl MessageTopicBroadcaster<AssistantStatusMessage> for VoiceAssistantService {}

impl PluginMetaGetter for VoiceAssistantService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for VoiceAssistantService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl MessageHandler<FfiEnvelopePayload<VoiceCommandMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<VoiceCommandMessage>, _sender_id: &str) {
        match message.0.action {
            VoiceCommandAction::Activate => {
                debug!("Voice Assistant: Activate command received");
                self.activate();
            }
            VoiceCommandAction::Deactivate => {
                debug!("Voice Assistant: Deactivate command received");
                self.deactivate();
            }
            VoiceCommandAction::SubmitText => {
                debug!("Voice Assistant: SubmitText command received: {}", message.0.text);
                self.submit_text(&message.0.text);
            }
            VoiceCommandAction::ClearConversation => {
                debug!("Voice Assistant: ClearConversation command received");
                self.clear_conversation();
            }
            VoiceCommandAction::EnableWakeWord => {
                debug!("Voice Assistant: EnableWakeWord command received");
                self.enable_wake_word();
            }
            VoiceCommandAction::DisableWakeWord => {
                debug!("Voice Assistant: DisableWakeWord command received");
                self.disable_wake_word();
            }
            VoiceCommandAction::SetWakeWordModel => {
                debug!("Voice Assistant: SetWakeWordModel command received: {}", message.text);
                // Parse the JSON and call set_wake_word_model
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&message.text) {
                    let model = json.get("model").and_then(|v| v.as_str()).unwrap_or("Alexa");
                    let threshold = json.get("threshold").and_then(|v| v.as_f64()).map(|t| t as f32);
                    self.set_wake_word_model(model, threshold);
                }
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<PersonalizationStatusMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<PersonalizationStatusMessage>, _sender_id: &str) {
        trace!("Voice Assistant: received personalization status");
        if let Ok(mut guard) = self.personalization.write() {
            *guard = Some(message.0);
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<smearor_doa_model::DoaStatusMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<smearor_doa_model::DoaStatusMessage>, _sender_id: &str) {
        let doa_status = message.0;

        // Update latest DoA angle and direction.
        if let Ok(mut angle) = self.doa_angle.write() {
            *angle = doa_status.calibrated_angle;
        }
        if let Ok(mut direction) = self.doa_direction.write() {
            *direction = doa_status.direction;
        }

        // Skip VAD edge detection if DoA VAD mode is disabled.
        if !self.config.doa_vad.enabled {
            return;
        }

        // TTS-Mute-Window: if TTS is speaking and AEC mirroring is not configured,
        // ignore VAD edges to prevent self-triggering from TTS output.
        if !self.config.doa_vad.aec_mirroring_enabled {
            if let Ok(speaking) = self.is_speaking.lock() {
                if *speaking {
                    // Reset edge detection state during TTS.
                    if let Ok(mut prev) = self.previous_speech_detected.lock() {
                        *prev = false;
                    }
                    if let Ok(mut onset) = self.vad_onset_timestamp.lock() {
                        *onset = None;
                    }
                    return;
                }
            }
        }

        let speech_detected = doa_status.speech_detected;
        let previous_speech = self.previous_speech_detected.lock().map(|p| *p).unwrap_or(false);

        if speech_detected && !previous_speech {
            // Rising edge: speech started.
            debug!("Voice Assistant: DoA VAD rising edge detected");
            if let Ok(mut onset) = self.vad_onset_timestamp.lock() {
                *onset = Some(std::time::Instant::now());
            }
            // Cancel any pending grace period exit.
            if let Ok(mut cancel) = self.vad_grace_cancel.lock() {
                if let Some(source_id) = cancel.take() {
                    source_id.remove();
                }
            }
        } else if speech_detected && previous_speech {
            // Continuous speech: check min_speech_duration_ms for activation.
            let should_activate = {
                let onset_opt = self.vad_onset_timestamp.lock().map(|o| *o).unwrap_or(None);
                if let Some(onset) = onset_opt {
                    let elapsed = onset.elapsed().as_millis() as u64;
                    elapsed >= self.config.doa_vad.min_speech_duration_ms
                } else {
                    false
                }
            };

            if should_activate {
                // Clear onset timestamp so we only activate once per rising edge.
                if let Ok(mut onset) = self.vad_onset_timestamp.lock() {
                    *onset = None;
                }

                // Check if already active.
                let is_active = self.active.lock().map(|a| *a).unwrap_or(false);
                if !is_active {
                    debug!(
                        "Voice Assistant: DoA VAD activating listening mode (angle={}, direction={})",
                        doa_status.calibrated_angle, doa_status.direction
                    );
                    self.activate();
                }
            }
        } else if !speech_detected && previous_speech {
            // Falling edge: speech stopped.
            debug!("Voice Assistant: DoA VAD falling edge detected, scheduling grace period exit");
            if let Ok(mut onset) = self.vad_onset_timestamp.lock() {
                *onset = None;
            }

            // Cancel any existing grace period timer.
            if let Ok(mut cancel) = self.vad_grace_cancel.lock() {
                if let Some(source_id) = cancel.take() {
                    source_id.remove();
                }
            }

            // Schedule a grace period exit using glib timeout (runs on main context,
            // no Tokio runtime required).
            let grace_period_ms = self.config.doa_vad.grace_period_ms;
            let active_clone = self.active.clone();
            let command_sender_clone = self.command_sender.clone();
            let vad_grace_cancel_clone = self.vad_grace_cancel.clone();

            let source_id = glib::source::timeout_add_local(std::time::Duration::from_millis(grace_period_ms), move || {
                // Clear the stored SourceId so the next rising edge doesn't
                // try to remove an already-consumed source.
                if let Ok(mut cancel) = vad_grace_cancel_clone.lock() {
                    cancel.take();
                }
                let is_active = active_clone.lock().map(|a| *a).unwrap_or(false);
                if is_active {
                    debug!("Voice Assistant: DoA VAD grace period expired, deactivating listening mode");
                    if let Some(sender) = &command_sender_clone {
                        let _ = sender.send(smearor_voice_assistant_model::VoiceCommandMessage::deactivate());
                    }
                }
                glib::ControlFlow::Break
            });

            if let Ok(mut cancel) = self.vad_grace_cancel.lock() {
                *cancel = Some(source_id);
            }
        }

        // Update previous speech detected state.
        if let Ok(mut prev) = self.previous_speech_detected.lock() {
            *prev = speech_detected;
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<RegisterToolMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterToolMessage>, _sender_id: &str) {
        let name = message.0.name.to_string();
        let description = message.0.description.to_string();
        let input_schema = message.0.input_schema.to_string();
        trace!("Voice Assistant: Tool registered: {}", name);
        self.on_tool_registered(name, description, input_schema);
    }
}

impl MessageHandler<FfiEnvelopePayload<RegisterResourceMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterResourceMessage>, _sender_id: &str) {
        let uri = message.0.uri.to_string();
        let name = message.0.name.to_string();
        let description = message.0.description.to_string();
        let mime_type = message.0.mime_type.to_string();
        trace!("Voice Assistant: Resource registered: {}", uri);
        self.on_resource_registered(uri, name, description, mime_type);
    }
}

impl MessageHandler<FfiEnvelopePayload<RegisterPromptMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterPromptMessage>, _sender_id: &str) {
        let name = message.0.name.to_string();
        let description = message.0.description.to_string();
        let arguments_schema = message.0.arguments_schema.to_string();
        let requires_memory = message.0.requires_memory;
        let memory_query = message.0.memory_query.to_string();
        let entity_filter = message.0.entity_filter.to_string();
        debug!("Voice Assistant: Prompt registered: {} (requires_memory={})", name, requires_memory);
        self.on_prompt_registered(name, description, arguments_schema, requires_memory, memory_query, entity_filter);
    }
}

impl ServicePlugin for VoiceAssistantService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<VoiceCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<VoiceCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_PERSONALIZATION_STATUS && envelope.type_id == FfiEnvelopePayload::<PersonalizationStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<PersonalizationStatusMessage>>::handle_envelope_message(self, envelope);
                } else if topic == smearor_doa_model::TOPIC_STATUS && envelope.type_id == FfiEnvelopePayload::<smearor_doa_model::DoaStatusMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_doa_model::DoaStatusMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_REGISTER_TOOL && envelope.type_id == FfiEnvelopePayload::<RegisterToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<RegisterToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_REGISTER_RESOURCE && envelope.type_id == FfiEnvelopePayload::<RegisterResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<RegisterResourceMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_REGISTER_PROMPT && envelope.type_id == FfiEnvelopePayload::<RegisterPromptMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<RegisterPromptMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT
                    && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokePromptMessage>::TYPE_ID
                {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokePromptMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_TOOL_RESPONSE && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokeToolResponse>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokeToolResponse>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_RESOURCE_RESPONSE && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokeResourceResponse>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokeResourceResponse>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_PROMPT_RESPONSE && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokePromptResponse>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokePromptResponse>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

fn broadcast_status(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, status: AssistantStatusMessage) {
    let payload_ptr = box_payload(status);
    let envelope = FfiEnvelope::builder()
        .sender_id(meta.id.clone())
        .target_instance_id("")
        .topic(AssistantStatusMessage::topic())
        .type_id(AssistantStatusMessage::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<AssistantStatusMessage>))
        .build();
    if let Some(context) = core_context {
        context.send_message(envelope);
    }
}
