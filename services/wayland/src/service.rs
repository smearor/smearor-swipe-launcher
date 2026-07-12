use crate::config::WaylandWorkspaceServiceConfig;
use crate::monitor::MonitorEvent;
use crate::monitor::spawn_monitor_worker;
use crate::workspace::WorkspaceEvent;
use crate::workspace::run_workspace_event_loop;
use crate::workspace::spawn_workspace_worker;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::WorkspaceSnapshotMessage;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::warn;

/// Internal command types the Wayland service handles asynchronously.
pub enum WaylandCommand {
    SwitchWorkspace(SwitchWorkspaceMessage),
    CreateWorkspace(CreateWorkspaceMessage),
    SnapshotRequest(WorkspaceSnapshotRequestMessage),
}

/// Shared workspace snapshot state, updated by the event worker and read by
/// the command handler for snapshot requests.
pub type SharedSnapshot = Arc<Mutex<Option<WorkspaceSnapshotMessage>>>;

/// Wayland workspace tracking service plugin.
///
/// Connects to the Wayland display and uses the `ext-workspace-unstable-v1`
/// protocol to detect workspace changes. Broadcasts `WorkspaceChangedEvent`
/// on `TOPIC_WORKSPACE_CHANGED` whenever the active workspace changes.
pub struct WaylandWorkspaceService {
    /// Plugin metadata.
    pub meta: PluginMeta,
    /// Optional core context for broadcasting messages.
    pub core_context: Option<FfiCoreContext>,
    /// Sender for commands into the async worker thread.
    pub command_sender: mpsc::UnboundedSender<WaylandCommand>,
    /// Shared snapshot state for responding to snapshot requests.
    pub shared_snapshot: SharedSnapshot,
    /// Shared configuration for the service.
    pub config: Arc<WaylandWorkspaceServiceConfig>,
}

impl WaylandWorkspaceService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        debug!(
            "Wayland workspace service: initializing, core_context is {}",
            if core_context.is_some() { "Some" } else { "None" }
        );

        smearor_model_compositor::register_json_converters(core_context);

        let service_config: WaylandWorkspaceServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (command_sender, mut command_receiver) = mpsc::unbounded_channel::<WaylandCommand>();
        let shared_snapshot: SharedSnapshot = Arc::new(Mutex::new(None));

        let service_config = Arc::new(service_config);
        let service = WaylandWorkspaceService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            command_sender,
            shared_snapshot: shared_snapshot.clone(),
            config: service_config,
        };

        let cmd_core_context = service.core_context;
        let cmd_shared_snapshot = shared_snapshot.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(error) => {
                    error!("Wayland service: failed to create tokio runtime: {error}");
                    return;
                }
            };

            rt.block_on(async move {
                while let Some(command) = command_receiver.recv().await {
                    match command {
                        WaylandCommand::SwitchWorkspace(message) => {
                            handle_switch_workspace(message).await;
                        }
                        WaylandCommand::CreateWorkspace(message) => {
                            handle_create_workspace(message).await;
                        }
                        WaylandCommand::SnapshotRequest(message) => {
                            handle_snapshot_request(message, cmd_core_context.clone(), &cmd_shared_snapshot).await;
                        }
                    }
                }
            });
        });

        if service.config.enable_workspace_tracking {
            let ws_core_context = service.core_context;
            let ws_meta = service.meta.clone();
            let mon_core_context = service.core_context;
            let mon_meta = service.meta.clone();
            let worker_snapshot = shared_snapshot.clone();

            let (workspace_sender, workspace_receiver) = mpsc::unbounded_channel::<WorkspaceEvent>();
            let (monitor_sender, monitor_receiver) = mpsc::unbounded_channel::<MonitorEvent>();

            std::thread::spawn(move || {
                run_workspace_event_loop(workspace_sender, monitor_sender);
            });

            spawn_workspace_worker(workspace_receiver, ws_core_context, ws_meta, worker_snapshot);

            spawn_monitor_worker(monitor_receiver, mon_core_context, mon_meta);
        }

        Ok(service)
    }
}

/// Handle a compositor-unified SwitchWorkspaceMessage.
///
/// The `ext-workspace-unstable-v1` protocol is read-only and does not support
/// switching workspaces. This is a compositor-specific operation.
async fn handle_switch_workspace(message: SwitchWorkspaceMessage) {
    warn!(
        "Wayland service: switching workspaces is not supported via the ext-workspace protocol (requested workspace {})",
        message.workspace_id
    );
}

/// Handle a compositor-unified CreateWorkspaceMessage.
///
/// The `ext-workspace-unstable-v1` protocol is read-only and does not support
/// creating workspaces. This is a compositor-specific operation.
async fn handle_create_workspace(message: CreateWorkspaceMessage) {
    warn!(
        "Wayland service: creating workspaces is not supported via the ext-workspace protocol (requested relative_to={}, position={:?})",
        message.relative_to, message.position
    );
}

/// Handle a WorkspaceSnapshotRequestMessage by reading the shared snapshot
/// state and broadcasting it.
async fn handle_snapshot_request(_message: WorkspaceSnapshotRequestMessage, core_context: Option<FfiCoreContext>, shared_snapshot: &SharedSnapshot) {
    debug!("Wayland service: handling workspace snapshot request");

    let snapshot = {
        let guard = shared_snapshot.lock();
        match guard.ok().and_then(|g| g.as_ref().cloned()) {
            Some(s) => s,
            None => {
                warn!("Wayland service: no workspace snapshot available yet");
                return;
            }
        }
    };

    let Some(ctx) = core_context else {
        return;
    };
    let broadcaster = MessageBroadcasterInner {
        meta: PluginMeta::new("wayland-service".to_string(), "Wayland Service".to_string(), None),
        core_context: Some(ctx),
    };
    broadcaster.broadcast_message_to_topic(snapshot);
}

impl MessageHandler<FfiEnvelopePayload<SwitchWorkspaceMessage>> for WaylandWorkspaceService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchWorkspaceMessage>, _sender_id: &str) {
        debug!("Wayland service: queueing workspace switch to {}", message.0.workspace_id);
        let _ = self.command_sender.send(WaylandCommand::SwitchWorkspace(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<CreateWorkspaceMessage>> for WaylandWorkspaceService {
    fn handle_message(&self, message: FfiEnvelopePayload<CreateWorkspaceMessage>, _sender_id: &str) {
        debug!(
            "Wayland service: queueing workspace creation relative_to={}, position={:?}",
            message.0.relative_to, message.0.position
        );
        let _ = self.command_sender.send(WaylandCommand::CreateWorkspace(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>> for WaylandWorkspaceService {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>, _sender_id: &str) {
        debug!("Wayland service: queueing workspace snapshot request");
        let _ = self.command_sender.send(WaylandCommand::SnapshotRequest(message.0));
    }
}

impl MessageBroadcaster for WaylandWorkspaceService {}

impl PluginMetaGetter for WaylandWorkspaceService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for WaylandWorkspaceService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for WaylandWorkspaceService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            debug!("Wayland service received message: topic={}, type_id={}", envelope.topic.to_string(), envelope.type_id);
            match envelope.type_id {
                id if id == FfiEnvelopePayload::<SwitchWorkspaceMessage>::TYPE_ID => {
                    debug!("SwitchWorkspaceMessage");
                    MessageHandler::<FfiEnvelopePayload<SwitchWorkspaceMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<CreateWorkspaceMessage>::TYPE_ID => {
                    debug!("CreateWorkspaceMessage");
                    MessageHandler::<FfiEnvelopePayload<CreateWorkspaceMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<WorkspaceSnapshotRequestMessage>::TYPE_ID => {
                    debug!("WorkspaceSnapshotRequestMessage");
                    MessageHandler::<FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>>::handle_envelope_message(self, envelope);
                }
                _ => {
                    warn!("Wayland service: unknown message type");
                }
            }
        }
    }
}
