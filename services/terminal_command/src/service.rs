use crate::config::TerminalCommandServiceConfig;
use glib::MainContext;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_terminal_command_model::TerminalCommandAction;
use smearor_terminal_command_model::TerminalCommandMessage;
use smearor_terminal_command_model::TerminalCommandStatusMessage;
use smearor_terminal_command_model::register_json_converters;
use smearor_wrot_process::ProcessConfig;
use smearor_wrot_process::ProcessExitEvent;
use smearor_wrot_process::ProcessManager;
use smearor_wrot_process::StdioConfig;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::trace;

pub struct TerminalCommandService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: TerminalCommandServiceConfig,
    pub process_manager: Arc<ProcessManager>,
}

impl TerminalCommandService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        register_json_converters(core_context.clone());

        let service_config: TerminalCommandServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let (exit_sender, exit_receiver) = mpsc::channel::<ProcessExitEvent>();
        let process_manager = Arc::new(ProcessManager::with_reaper(Duration::from_secs(2), exit_sender));

        let service = TerminalCommandService {
            meta: PluginMeta::try_from(&config)?,
            config: service_config,
            core_context,
            process_manager,
        };

        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<ProcessExitEvent>();
        std::thread::spawn(move || {
            while let Ok(event) = exit_receiver.recv() {
                if event_tx.send(event).is_err() {
                    break;
                }
            }
        });

        let broadcaster = service.get_broadcaster();
        let commands = Arc::new(service.config.commands.clone());
        let process_manager_for_restart = service.process_manager.clone();
        MainContext::default().spawn_local(async move {
            while let Some(event) = event_rx.recv().await {
                debug!("TerminalCommand Service: Process exited: label={}, pid={}", event.label, event.pid);
                broadcaster.broadcast_message_to_topic(TerminalCommandStatusMessage::stopped(&event.label));
                if event.restart_on_exit
                    && let Some(definition) = commands.get(&event.label)
                {
                    debug!("TerminalCommand Service: Restarting command: {}", event.label);
                    let config = ProcessConfig::builder()
                        .command(definition.command.clone())
                        .args(definition.args.clone())
                        .env(definition.env.clone())
                        .working_dir(definition.working_dir.clone())
                        .kill_signal(definition.kill_signal)
                        .terminate_timeout_ms(definition.terminate_timeout_ms)
                        .restart_on_exit(definition.restart_on_exit)
                        .stdin(StdioConfig::Null)
                        .stdout(StdioConfig::Null)
                        .stderr(StdioConfig::Null)
                        .build();
                    match process_manager_for_restart.start(&event.label, &config) {
                        Ok(id) => {
                            debug!("TerminalCommand Service: Restarted command {} with process id {}", event.label, id);
                            let pids = process_manager_for_restart.pids_by_label(&event.label);
                            if let Some(pid) = pids.first() {
                                broadcaster.broadcast_message_to_topic(TerminalCommandStatusMessage::running(&event.label, *pid));
                            }
                        }
                        Err(e) => {
                            error!("TerminalCommand Service: Failed to restart command {}: {}", event.label, e);
                            broadcaster.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(&event.label));
                        }
                    }
                }
            }
        });

        service.register_mcp_capabilities();
        Ok(service)
    }

    /// Launches a configured command by `command_id`.
    pub(crate) fn handle_launch(&self, command_id: &str, forked: bool, terminate_on_exit: bool) {
        trace!("TerminalCommand Service: Launching command: {command_id} (forked={forked})");

        let Some(definition) = self.config.commands.get(command_id) else {
            error!("TerminalCommand Service: Unknown command_id: {command_id}");
            self.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(command_id));
            return;
        };

        if let Some(working_dir) = &definition.working_dir {
            if !working_dir.is_dir() {
                error!("TerminalCommand Service: working_dir is not a directory: {:?}", working_dir);
                self.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(command_id));
                return;
            }
        }

        let config = ProcessConfig::builder()
            .command(definition.command.clone())
            .args(definition.args.clone())
            .env(definition.env.clone())
            .working_dir(definition.working_dir.clone())
            .forked(forked)
            .terminate_on_exit(terminate_on_exit)
            .kill_signal(definition.kill_signal)
            .terminate_timeout_ms(definition.terminate_timeout_ms)
            .restart_on_exit(definition.restart_on_exit)
            .stdin(StdioConfig::Null)
            .stdout(StdioConfig::Null)
            .stderr(StdioConfig::Null)
            .build();

        match self.process_manager.start(command_id, &config) {
            Ok(id) => {
                let pid = self.process_manager.get(id).map(|p| p.pid).unwrap_or(0);
                debug!("TerminalCommand Service: Successfully spawned {} with PID {} (forked={forked})", definition.command, pid);
                if !forked {
                    self.broadcast_message_to_topic(TerminalCommandStatusMessage::running(command_id, pid));
                } else {
                    debug!("TerminalCommand Service: Process PID {} is forked/detached, not broadcasting running status", pid);
                }
            }
            Err(e) => {
                error!("TerminalCommand Service: Failed to spawn command {}: {}", definition.command, e);
                self.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(command_id));
            }
        }
    }

    /// Terminates all tracked processes for the given `command_id`.
    pub(crate) fn handle_terminate(&self, command_id: &str) {
        trace!("TerminalCommand Service: Terminating command: {command_id}");

        if self.config.commands.get(command_id).is_none() {
            debug!("TerminalCommand Service: Unknown command_id for terminate: {command_id}");
            self.broadcast_message_to_topic(TerminalCommandStatusMessage::stopped(command_id));
            return;
        }

        match self.process_manager.stop_label(command_id) {
            Ok(_) => {
                debug!("TerminalCommand Service: Successfully terminated command: {command_id}");
            }
            Err(e) => {
                error!("TerminalCommand Service: Failed to terminate command {command_id}: {}", e);
            }
        }
        self.broadcast_message_to_topic(TerminalCommandStatusMessage::stopped(command_id));
    }

    /// Restart a command: terminate then launch.
    pub(crate) fn handle_restart(&self, command_id: &str, forked: bool, terminate_on_exit: bool) {
        trace!("TerminalCommand Service: Restarting command: {command_id}");
        self.handle_terminate(command_id);
        self.handle_launch(command_id, forked, terminate_on_exit);
    }
}

impl MessageHandler<FfiEnvelopePayload<TerminalCommandMessage>> for TerminalCommandService {
    fn handle_message(&self, message: FfiEnvelopePayload<TerminalCommandMessage>, _sender_id: &str) {
        trace!("handle_message: {message:?}");
        match message.action {
            TerminalCommandAction::Launch => {
                self.handle_launch(&message.command_id, message.forked, message.terminate_on_exit);
            }
            TerminalCommandAction::Terminate => {
                self.handle_terminate(&message.command_id);
            }
            TerminalCommandAction::Restart => {
                self.handle_restart(&message.command_id, message.forked, message.terminate_on_exit);
            }
        }
    }
}

impl MessageBroadcaster for TerminalCommandService {}

impl MessageTopicBroadcaster<TerminalCommandStatusMessage> for TerminalCommandService {}

impl PluginMetaGetter for TerminalCommandService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for TerminalCommandService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl ServicePlugin for TerminalCommandService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<TerminalCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<TerminalCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_PROMPT && envelope.type_id == FfiEnvelopePayload::<InvokePromptMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokePromptMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}
