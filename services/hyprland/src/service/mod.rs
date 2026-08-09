mod command;
mod converters;
mod ctl;
mod dispatch;
pub(crate) mod instance_signature;
pub(crate) mod message_handlers;
mod shared_state;
mod state;

pub use command::HyprlandCommand;
pub(crate) use instance_signature::ensure_hyprland_instance_signature;
pub use shared_state::HyprlandSharedState;

use crate::config::HyprlandServiceConfig;
use crate::event_listener::spawn_event_listener;
use crate::event_listener::spawn_event_worker;
use command::spawn_command_worker;
use smearor_hyprland_model::HyprlandStateMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::debug;

pub struct HyprlandService {
    /// Plugin metadata.
    pub meta: PluginMeta,
    /// Optional core context for broadcasting messages.
    pub core_context: Option<FfiCoreContext>,
    /// Sender for commands into the async worker thread.
    pub command_sender: mpsc::UnboundedSender<HyprlandCommand>,
    /// Shared configuration for the service.
    pub config: Arc<HyprlandServiceConfig>,
    /// Shared state for MCP resource queries (cached events and snapshots).
    pub shared_state: Arc<Mutex<HyprlandSharedState>>,
}

impl HyprlandService {
    pub(crate) fn status_snapshot(&self) -> Option<HyprlandStateMessage> {
        self.shared_state.lock().ok().and_then(|s| s.last_state.clone())
    }

    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        debug!(
            "Hyprland service: registering JSON converters, core_context is {}",
            if core_context.is_some() { "Some" } else { "None" }
        );
        smearor_hyprland_model::register_json_converters(core_context);
        smearor_model_compositor::register_json_converters(core_context);
        debug!("Hyprland service: JSON converters registered");

        let service_config: HyprlandServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (command_sender, command_receiver) = mpsc::unbounded_channel::<HyprlandCommand>();

        let service_config = Arc::new(service_config);
        let shared_state = Arc::new(Mutex::new(HyprlandSharedState::default()));
        let service = HyprlandService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            command_sender,
            config: service_config,
            shared_state,
        };
        service.register_mcp_capabilities();

        spawn_command_worker(command_receiver, service.core_context.clone(), service.meta.clone(), Arc::clone(&service.shared_state));
        service.spawn_event_listeners();

        // Request initial state so last_state is populated for MCP resource queries
        let _ = service.command_sender.send(HyprlandCommand::StateRequest);

        Ok(service)
    }

    fn spawn_event_listeners(&self) {
        debug!(
            "Hyprland service config: enable_workspace_tracking={}, enable_monitor_events={}, enable_status_events={}, enable_workspace_lifecycle={}",
            self.config.enable_workspace_tracking, self.config.enable_monitor_events, self.config.enable_status_events, self.config.enable_workspace_lifecycle
        );
        if self.config.enable_workspace_tracking || self.config.enable_monitor_events || self.config.enable_status_events {
            let (event_sender, event_receiver) = mpsc::unbounded_channel();
            spawn_event_listener(
                event_sender,
                self.config.enable_workspace_tracking,
                self.config.enable_monitor_events,
                self.config.enable_status_events,
            );
            spawn_event_worker(
                event_receiver,
                self.core_context.clone(),
                self.meta.clone(),
                self.config.enable_workspace_lifecycle,
                Arc::clone(&self.shared_state),
            );
        }
    }
}
