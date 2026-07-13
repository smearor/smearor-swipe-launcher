use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use glib::MainContext;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_model_mcp::TOPIC_MCP_REGISTER_TOOL;
use smearor_model_mcp::TOPIC_MCP_TOOL_RESPONSE;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_voice_assistant_model::AssistantState;
use smearor_voice_assistant_model::AssistantStatusMessage;
use smearor_voice_assistant_model::ToolCatalogEntry;
use smearor_voice_assistant_model::VoiceCommandAction;
use smearor_voice_assistant_model::VoiceCommandMessage;
use tracing::debug;
use tracing::error;
use whisper_rs::WhisperContext;

use crate::audio::capture_audio;
use crate::config::VoiceAssistantServiceConfig;
use crate::llm::LlmInferenceEngine;
use crate::react::PendingInvocations;
use crate::transcriber::load_whisper_context;
use crate::transcriber::transcribe_async;

pub struct VoiceAssistantService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: VoiceAssistantServiceConfig,
    pub state: Arc<RwLock<AssistantState>>,
    pub tool_catalog: Arc<RwLock<Vec<ToolCatalogEntry>>>,
    pub whisper_context: Option<Arc<WhisperContext>>,
    pub llm_engine: Option<Arc<LlmInferenceEngine>>,
    pub pending_invocations: PendingInvocations,
    pub current_transcript: Arc<RwLock<String>>,
    pub current_answer: Arc<RwLock<String>>,
    pub active: Arc<Mutex<bool>>,
    pub command_sender: Option<tokio::sync::mpsc::UnboundedSender<VoiceCommandMessage>>,
    pub status_sender: Option<tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>>,
}

impl VoiceAssistantService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_voice_assistant_model::register_json_converters(core_context);

        let service_config: VoiceAssistantServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let mut service = VoiceAssistantService {
            meta: PluginMeta::try_from(&config)?,
            config: service_config,
            core_context,
            state: Arc::new(RwLock::new(AssistantState::Idle)),
            tool_catalog: Arc::new(RwLock::new(Vec::new())),
            whisper_context: None,
            llm_engine: None,
            pending_invocations: Arc::new(Mutex::new(std::collections::HashMap::new())),
            current_transcript: Arc::new(RwLock::new(String::new())),
            current_answer: Arc::new(RwLock::new(String::new())),
            active: Arc::new(Mutex::new(false)),
            command_sender: None,
            status_sender: None,
        };

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

        // Initialize LLM engine.
        let llm_config = service.config.to_llm_config();
        match LlmInferenceEngine::load(&llm_config) {
            Ok(engine) => {
                debug!("Voice Assistant: LLM engine loaded");
                service.llm_engine = Some(Arc::new(engine));
            }
            Err(error) => {
                error!("Voice Assistant: Failed to load LLM engine: {error}");
            }
        }

        // Set up status broadcast channel.
        let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<AssistantStatusMessage>();
        let broadcaster = service.get_broadcaster();
        MainContext::default().spawn_local(async move {
            while let Some(status) = status_receiver.recv().await {
                broadcaster.broadcast_message_to_topic(status);
            }
        });

        // Spawn the async pipeline handler.
        let (command_sender, mut command_receiver) = tokio::sync::mpsc::unbounded_channel::<VoiceCommandMessage>();
        let service_state = service.state.clone();
        let service_whisper = service.whisper_context.clone();
        let service_llm = service.llm_engine.clone();
        let service_config = service.config.clone();
        let service_transcript = service.current_transcript.clone();
        let service_answer = service.current_answer.clone();
        let service_active = service.active.clone();
        let service_pending = service.pending_invocations.clone();
        let service_tool_catalog = service.tool_catalog.clone();
        let service_core_context = service.core_context.clone();
        let service_meta = service.meta.clone();
        let service_status_sender = status_sender.clone();

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

                            // Run the pipeline.
                            Self::run_pipeline_inner(
                                &service_config,
                                &service_state,
                                &service_whisper,
                                &service_llm,
                                &service_transcript,
                                &service_answer,
                                &service_active,
                                &service_pending,
                                &service_tool_catalog,
                                &service_core_context,
                                &service_meta,
                                &service_status_sender,
                            )
                            .await;
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
                                &service_transcript,
                                &service_answer,
                                &service_active,
                                &service_pending,
                                &service_tool_catalog,
                                &service_core_context,
                                &service_meta,
                                &service_status_sender,
                            )
                            .await;
                        }
                    }
                }
            });
        });

        // Store the command sender for later use.
        service.command_sender = Some(command_sender);
        service.status_sender = Some(status_sender);

        service.register_mcp_capabilities();
        Ok(service)
    }

    /// Sets the assistant state and broadcasts a status update.
    async fn set_state(
        state: &Arc<RwLock<AssistantState>>,
        new_state: AssistantState,
        status_sender: &tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
        transcript: &Arc<RwLock<String>>,
        answer: &Arc<RwLock<String>>,
    ) {
        if let Ok(mut state_guard) = state.write() {
            *state_guard = new_state.clone();
        }
        let current_transcript = transcript.read().map(|t| t.clone()).unwrap_or_default();
        let current_answer = answer.read().map(|a| a.clone()).unwrap_or_default();
        let status = AssistantStatusMessage::new(new_state);
        let _ = status_sender.send(status);
    }

    /// Runs the complete voice pipeline: capture -> STT -> ReAct -> status.
    async fn run_pipeline_inner(
        config: &VoiceAssistantServiceConfig,
        state: &Arc<RwLock<AssistantState>>,
        whisper_context: &Option<Arc<WhisperContext>>,
        llm_engine: &Option<Arc<LlmInferenceEngine>>,
        transcript: &Arc<RwLock<String>>,
        answer: &Arc<RwLock<String>>,
        active: &Arc<Mutex<bool>>,
        pending_invocations: &PendingInvocations,
        tool_catalog: &Arc<RwLock<Vec<ToolCatalogEntry>>>,
        core_context: &Option<FfiCoreContext>,
        meta: &PluginMeta,
        status_sender: &tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
    ) {
        // 1. Capture audio.
        Self::set_state(state, AssistantState::Listening, status_sender, transcript, answer).await;

        let (_stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let samples = match capture_audio(config, stop_rx).await {
            Ok(samples) => samples,
            Err(error) => {
                error!("Voice Assistant: Audio capture failed: {error}");
                Self::set_error(state, &error.to_string(), transcript, answer).await;
                let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
                *active_guard = false;
                return;
            }
        };

        // 2. Transcribe.
        Self::set_state(state, AssistantState::ProcessingStt, status_sender, transcript, answer).await;

        let whisper_ctx = match whisper_context {
            Some(ctx) => ctx.clone(),
            None => {
                error!("Voice Assistant: Whisper context not initialized");
                Self::set_error(state, "Whisper context not initialized", transcript, answer).await;
                let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
                *active_guard = false;
                return;
            }
        };

        let transcribed = match transcribe_async(whisper_ctx, samples, config.language.clone()).await {
            Ok(text) => text,
            Err(error) => {
                error!("Voice Assistant: STT failed: {error}");
                Self::set_error(state, &error.to_string(), transcript, answer).await;
                let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
                *active_guard = false;
                return;
            }
        };

        debug!("Voice Assistant: Transcribed: {}", transcribed);
        if let Ok(mut transcript_guard) = transcript.write() {
            *transcript_guard = transcribed.clone();
        }

        // 3. ReAct loop.
        Self::run_react(
            &transcribed,
            config,
            state,
            llm_engine,
            transcript,
            answer,
            active,
            pending_invocations,
            tool_catalog,
            core_context,
            meta,
            status_sender,
        )
        .await;
    }

    /// Runs the text pipeline (bypasses STT).
    async fn run_text_pipeline(
        text: &str,
        config: &VoiceAssistantServiceConfig,
        state: &Arc<RwLock<AssistantState>>,
        llm_engine: &Option<Arc<LlmInferenceEngine>>,
        transcript: &Arc<RwLock<String>>,
        answer: &Arc<RwLock<String>>,
        active: &Arc<Mutex<bool>>,
        pending_invocations: &PendingInvocations,
        tool_catalog: &Arc<RwLock<Vec<ToolCatalogEntry>>>,
        core_context: &Option<FfiCoreContext>,
        meta: &PluginMeta,
        status_sender: &tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
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
            transcript,
            answer,
            active,
            pending_invocations,
            tool_catalog,
            core_context,
            meta,
            status_sender,
        )
        .await;
    }

    /// Runs the ReAct loop and handles the result.
    async fn run_react(
        user_text: &str,
        config: &VoiceAssistantServiceConfig,
        state: &Arc<RwLock<AssistantState>>,
        llm_engine: &Option<Arc<LlmInferenceEngine>>,
        transcript: &Arc<RwLock<String>>,
        answer: &Arc<RwLock<String>>,
        active: &Arc<Mutex<bool>>,
        pending_invocations: &PendingInvocations,
        tool_catalog: &Arc<RwLock<Vec<ToolCatalogEntry>>>,
        core_context: &Option<FfiCoreContext>,
        meta: &PluginMeta,
        status_sender: &tokio::sync::mpsc::UnboundedSender<AssistantStatusMessage>,
    ) {
        // Create a temporary service reference for the ReAct loop.
        let temp_service = VoiceAssistantService {
            meta: meta.clone(),
            core_context: core_context.clone(),
            config: config.clone(),
            state: state.clone(),
            tool_catalog: tool_catalog.clone(),
            whisper_context: None,
            llm_engine: llm_engine.clone(),
            pending_invocations: pending_invocations.clone(),
            current_transcript: transcript.clone(),
            current_answer: answer.clone(),
            active: active.clone(),
            command_sender: None,
            status_sender: None,
        };

        Self::set_state(state, AssistantState::ThinkingLlm, status_sender, transcript, answer).await;

        match temp_service.execute_react_loop(user_text).await {
            Ok(final_answer) => {
                debug!("Voice Assistant: Final answer: {}", final_answer);
                if let Ok(mut answer_guard) = answer.write() {
                    *answer_guard = final_answer.clone();
                }
                Self::set_state(state, AssistantState::Idle, status_sender, transcript, answer).await;
            }
            Err(error) => {
                error!("Voice Assistant: ReAct loop failed: {error}");
                Self::set_error(state, &error.to_string(), transcript, answer).await;
            }
        }

        let mut active_guard = active.lock().unwrap_or_else(|e| e.into_inner());
        *active_guard = false;
    }

    /// Sets the error state and broadcasts a status update.
    async fn set_error(state: &Arc<RwLock<AssistantState>>, error_message: &str, transcript: &Arc<RwLock<String>>, answer: &Arc<RwLock<String>>) {
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
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<RegisterToolMessage>> for VoiceAssistantService {
    fn handle_message(&self, message: FfiEnvelopePayload<RegisterToolMessage>, _sender_id: &str) {
        let name = message.0.name.to_string();
        let description = message.0.description.to_string();
        let input_schema = message.0.input_schema.to_string();
        debug!("Voice Assistant: Tool registered: {}", name);
        self.on_tool_registered(name, description, input_schema);
    }
}

impl Service for VoiceAssistantService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<VoiceCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<VoiceCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_REGISTER_TOOL && envelope.type_id == FfiEnvelopePayload::<RegisterToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<RegisterToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_TOOL_RESPONSE && envelope.type_id == FfiEnvelopePayload::<smearor_model_mcp::InvokeToolResponse>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<smearor_model_mcp::InvokeToolResponse>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}
