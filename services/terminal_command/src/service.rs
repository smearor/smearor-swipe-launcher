pub(crate) mod launch;
pub(crate) mod reaper;
pub(crate) mod terminate;
pub(crate) mod tracked_process;

use crate::config::TerminalCommandServiceConfig;
use dashmap::DashMap;
use glib::MainContext;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
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
use smearor_terminal_command_model::TerminalCommandAction;
use smearor_terminal_command_model::TerminalCommandMessage;
use smearor_terminal_command_model::TerminalCommandStatusMessage;
use smearor_terminal_command_model::register_json_converters;
use std::path::Path;
use std::sync::Arc;
use tracing::debug;
use tracing::error;
use tracing::trace;
use which::which;

pub use tracked_process::TrackedProcess;

pub struct TerminalCommandService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: TerminalCommandServiceConfig,
    pub tracked_processes: Arc<DashMap<String, Vec<TrackedProcess>>>,
}

impl TerminalCommandService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        register_json_converters(core_context.clone());

        let service_config: TerminalCommandServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let service = TerminalCommandService {
            meta: PluginMeta::try_from(&config)?,
            config: service_config,
            core_context,
            tracked_processes: Arc::new(DashMap::new()),
        };

        let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<TerminalCommandStatusMessage>();
        let (restart_sender, mut restart_receiver) = tokio::sync::mpsc::unbounded_channel::<String>();

        reaper::spawn_reaper_thread(service.tracked_processes.clone(), Arc::new(service.config.commands.clone()), status_sender, restart_sender);

        let broadcaster = service.get_broadcaster();
        MainContext::default().spawn_local(async move {
            while let Some(status) = status_receiver.recv().await {
                broadcaster.broadcast_message_to_topic(status);
            }
        });

        let restart_broadcaster = service.get_broadcaster();
        MainContext::default().spawn_local(async move {
            while let Some(command_id) = restart_receiver.recv().await {
                debug!("TerminalCommand Service: Restarting command via broker: {}", command_id);
                restart_broadcaster.broadcast_message("service.terminal_command.command", &TerminalCommandMessage::launch(&command_id, false, true));
            }
        });

        service.register_mcp_capabilities();
        Ok(service)
    }

    /// Restart a command: terminate then launch.
    pub(crate) fn handle_restart(&self, command_id: &str, forked: bool, terminate_on_exit: bool) {
        trace!("TerminalCommand Service: Restarting command: {command_id}");
        self.handle_terminate(command_id);
        self.handle_launch(command_id, forked, terminate_on_exit);
    }

    /// Resolve a command name to an absolute path via `$PATH`.
    pub fn resolve_command(&self, command: &str) -> String {
        if Path::new(command).is_absolute() {
            return command.to_string();
        }
        which(command)
            .map(|path| {
                trace!("Resolved command '{command}' to: {path:?}");
                path.to_string_lossy().to_string()
            })
            .unwrap_or_else(|e| {
                trace!("Failed to resolve command '{command}': {}", e);
                String::new()
            })
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

impl Service for TerminalCommandService {
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
                }
            }
        }
    }
}

impl Drop for TerminalCommandService {
    fn drop(&mut self) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            debug!("TerminalCommand Service: Dropping service, terminating all tracked processes");
            for entry in self.tracked_processes.iter() {
                for tp in entry.value().iter() {
                    if !tp.terminate_on_exit {
                        continue;
                    }
                    let proc_path = format!("/proc/{}", tp.pid);
                    if Path::new(&proc_path).exists() {
                        debug!("TerminalCommand Service: Sending SIGTERM to process {} on drop", tp.pid);
                        let posix_pid = Pid::from_raw(tp.pid as i32);
                        if let Err(e) = kill(posix_pid, Signal::SIGTERM) {
                            error!("TerminalCommand Service: Failed to kill process {} on drop: {}", tp.pid, e);
                        }
                    }
                }
            }
        }));

        if let Err(_) = result {
            error!("TerminalCommand Service: panic during drop, processes may not have been terminated");
        }
    }
}
