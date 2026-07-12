use crate::config::GnomeWorkspaceServiceConfig;
use crate::monitor::MonitorEvent;
use crate::monitor::run_monitor_polling;
use crate::monitor::spawn_monitor_worker;
use crate::workspace::WorkspaceEvent;
use crate::workspace::run_workspace_polling;
use crate::workspace::spawn_workspace_worker;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::WorkspaceCreatePosition;
use smearor_model_compositor::WorkspaceInfo;
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
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::warn;

/// Internal command types the GNOME service handles asynchronously.
pub enum GnomeCommand {
    SwitchWorkspace(SwitchWorkspaceMessage),
    CreateWorkspace(CreateWorkspaceMessage),
    SnapshotRequest(WorkspaceSnapshotRequestMessage),
}

/// GNOME workspace tracking service plugin.
///
/// Uses `org.gnome.Shell.Introspect.GetWindows` to poll the active workspace
/// every 500ms (configurable). Workspace names and counts are read from
/// GSettings (`org.gnome.desktop.wm.preferences`). When the workspace changes,
/// resolves the monitor index via `org.gnome.Mutter.DisplayConfig` and
/// broadcasts a `WorkspaceChangedEvent` on `TOPIC_WORKSPACE_CHANGED`.
///
/// Workspace switching and creation still use `org.gnome.Shell.Eval`, which
/// requires GNOME Shell unsafe mode on GNOME 41+.
pub struct GnomeWorkspaceService {
    /// Plugin metadata.
    pub meta: PluginMeta,
    /// Optional core context for broadcasting messages.
    pub core_context: Option<FfiCoreContext>,
    /// Sender for commands into the async worker thread.
    pub command_sender: mpsc::UnboundedSender<GnomeCommand>,
    /// Shared configuration for the service.
    pub config: Arc<GnomeWorkspaceServiceConfig>,
}

impl GnomeWorkspaceService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        debug!(
            "GNOME workspace service: initializing, core_context is {}",
            if core_context.is_some() { "Some" } else { "None" }
        );

        smearor_model_compositor::register_json_converters(core_context);

        let service_config: GnomeWorkspaceServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (command_sender, mut command_receiver) = mpsc::unbounded_channel::<GnomeCommand>();

        let service_config = Arc::new(service_config);
        let service = GnomeWorkspaceService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            command_sender,
            config: service_config,
        };

        let cmd_core_context = service.core_context;
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(error) => {
                    error!("GNOME service: failed to create tokio runtime: {error}");
                    return;
                }
            };

            rt.block_on(async move {
                while let Some(command) = command_receiver.recv().await {
                    match command {
                        GnomeCommand::SwitchWorkspace(message) => {
                            handle_switch_workspace(message).await;
                        }
                        GnomeCommand::CreateWorkspace(message) => {
                            handle_create_workspace(message).await;
                        }
                        GnomeCommand::SnapshotRequest(message) => {
                            handle_snapshot_request(message, cmd_core_context.clone()).await;
                        }
                    }
                }
            });
        });

        if service.config.enable_workspace_tracking {
            let ws_core_context = service.core_context;
            let ws_meta = service.meta.clone();
            let poll_interval = service.config.poll_interval_ms;
            let enable_workspace_lifecycle = service.config.enable_workspace_lifecycle;

            let (ws_sender, ws_receiver) = mpsc::unbounded_channel::<WorkspaceEvent>();

            std::thread::spawn(move || {
                run_workspace_polling(ws_sender, poll_interval, enable_workspace_lifecycle);
            });

            spawn_workspace_worker(ws_receiver, ws_core_context, ws_meta);
        }

        if service.config.enable_monitor_events {
            let mon_core_context = service.core_context;
            let mon_meta = service.meta.clone();
            let poll_interval = service.config.poll_interval_ms;

            let (mon_sender, mon_receiver) = mpsc::unbounded_channel::<MonitorEvent>();

            std::thread::spawn(move || {
                run_monitor_polling(mon_sender, poll_interval);
            });

            spawn_monitor_worker(mon_receiver, mon_core_context, mon_meta);
        }

        Ok(service)
    }
}

/// Handle a compositor-unified SwitchWorkspaceMessage via GNOME Shell Eval.
///
/// Note: On GNOME 41+, `Eval` requires unsafe mode. If unsafe mode is not
/// enabled, this will log a warning and fail silently.
async fn handle_switch_workspace(message: SwitchWorkspaceMessage) {
    debug!("GNOME service: switching to workspace {}", message.workspace_id);
    let js_code = format!("global.workspace_manager.get_workspace_by_index({}).activate()", message.workspace_id);
    match eval_shell(&js_code).await {
        Ok((success, result)) => {
            if !success {
                warn!("GNOME service: Shell.Eval returned failure for switch (enable unsafe mode): {result}");
            }
        }
        Err(error) => {
            error!("GNOME service: failed to switch workspace: {error}");
        }
    }
}

/// Handle a compositor-unified CreateWorkspaceMessage via GNOME Shell Eval.
///
/// GNOME supports `append_new_workspace()` and `prepend_new_workspace()`.
/// For `After`, we append. For `Before`, we prepend.
///
/// Note: On GNOME 41+, `Eval` requires unsafe mode.
async fn handle_create_workspace(message: CreateWorkspaceMessage) {
    debug!("GNOME service: creating workspace relative_to={}, position={:?}", message.relative_to, message.position);
    let js_code = match message.position {
        WorkspaceCreatePosition::After => "global.workspace_manager.append_new_workspace(false)".to_string(),
        WorkspaceCreatePosition::Before => "global.workspace_manager.prepend_new_workspace(false)".to_string(),
    };
    match eval_shell(&js_code).await {
        Ok((success, result)) => {
            if !success {
                warn!("GNOME service: Shell.Eval returned failure for create: {result}");
            }
        }
        Err(error) => {
            error!("GNOME service: failed to create workspace: {error}");
        }
    }
}

/// Handle a WorkspaceSnapshotRequestMessage by querying GNOME Shell for all
/// workspaces and broadcasting a WorkspaceSnapshotMessage.
///
/// Tries Introspect.GetWindows first, then Shell.Eval, then GSettings-only.
async fn handle_snapshot_request(_message: WorkspaceSnapshotRequestMessage, core_context: Option<FfiCoreContext>) {
    debug!("GNOME service: building workspace snapshot");

    let connection = match zbus::Connection::session().await {
        Ok(conn) => conn,
        Err(error) => {
            error!("GNOME service: failed to connect to D-Bus for snapshot: {error}");
            return;
        }
    };

    // Try Introspect.GetWindows first.
    let (active_workspace, max_workspace) = if let Ok(introspect_proxy) = crate::workspace::dbus::GnomeShellIntrospectProxy::new(&connection).await {
        match introspect_proxy.get_windows().await {
            Ok(windows) => {
                debug!("GNOME service: snapshot using Introspect.GetWindows");
                analyze_windows_for_snapshot(&windows)
            }
            Err(error) => {
                debug!("GNOME service: Introspect.GetWindows blocked for snapshot ({error}), trying Shell.Eval");
                (None, 0)
            }
        }
    } else {
        (None, 0)
    };

    // If Introspect didn't work, try Shell.Eval for the active workspace.
    let (active_workspace, max_workspace) = if active_workspace.is_none() {
        if let Ok(eval_proxy) = crate::workspace::dbus::GnomeShellEvalProxy::new(&connection).await {
            match eval_proxy.eval("global.workspace_manager.get_active_workspace_index()").await {
                Ok((true, result)) => {
                    let active = result.trim().parse::<i32>().ok().unwrap_or(0);
                    debug!("GNOME service: snapshot using Shell.Eval (active={})", active);
                    let count = if crate::workspace::gsettings::is_dynamic_workspaces() {
                        active + 1
                    } else {
                        crate::workspace::gsettings::read_workspace_count()
                    };
                    (Some(active), count.saturating_sub(1))
                }
                _ => {
                    debug!("GNOME service: Shell.Eval blocked for snapshot, using GSettings-only");
                    let count = crate::workspace::gsettings::read_workspace_count();
                    (Some(0), count.saturating_sub(1))
                }
            }
        } else {
            let count = crate::workspace::gsettings::read_workspace_count();
            (Some(0), count.saturating_sub(1))
        }
    } else {
        (active_workspace, max_workspace)
    };

    let active_id = active_workspace.unwrap_or(0);
    let ws_list = crate::workspace::gsettings::build_workspace_list(max_workspace + 1, active_id);

    let snapshot = WorkspaceSnapshotMessage {
        workspaces: ws_list
            .into_iter()
            .map(|(id, name)| WorkspaceInfo {
                workspace_id: id,
                workspace_name: name.into(),
                monitor_index: 0,
                is_active: id == active_id,
            })
            .collect(),
        active_workspace_id: active_id,
        active_monitor_index: 0,
    };

    debug!(
        "GNOME service: snapshot built with {} workspaces, active={}",
        snapshot.workspaces.len(),
        snapshot.active_workspace_id
    );

    let Some(ctx) = core_context else {
        return;
    };
    let broadcaster = MessageBroadcasterInner {
        meta: PluginMeta::new("gnome-service".to_string(), "GNOME Service".to_string(), None),
        core_context: Some(ctx),
    };
    broadcaster.broadcast_message_to_topic(snapshot);
}

/// Analyze the Introspect window map for snapshot requests.
///
/// Returns `(active_workspace, max_workspace)`.
fn analyze_windows_for_snapshot(
    windows: &std::collections::HashMap<String, std::collections::HashMap<String, zbus::zvariant::OwnedValue>>,
) -> (Option<i32>, i32) {
    let mut active_workspace: Option<i32> = None;
    let mut max_workspace: i32 = 0;

    for props in windows.values() {
        if let Some(workspace_value) = props.get("workspace") {
            let ws_id = match &**workspace_value {
                zbus::zvariant::Value::I32(v) => *v,
                zbus::zvariant::Value::I64(v) => *v as i32,
                zbus::zvariant::Value::U32(v) => *v as i32,
                zbus::zvariant::Value::U64(v) => *v as i32,
                _ => workspace_value.to_string().trim().parse::<i32>().ok().unwrap_or(0),
            };
            if ws_id > max_workspace {
                max_workspace = ws_id;
            }
        }

        if let Some(focus_value) = props.get("has-focus") {
            if matches!(&**focus_value, zbus::zvariant::Value::Bool(true)) {
                if let Some(workspace_value) = props.get("workspace") {
                    active_workspace = Some(match &**workspace_value {
                        zbus::zvariant::Value::I32(v) => *v,
                        zbus::zvariant::Value::I64(v) => *v as i32,
                        zbus::zvariant::Value::U32(v) => *v as i32,
                        zbus::zvariant::Value::U64(v) => *v as i32,
                        _ => workspace_value.to_string().trim().parse::<i32>().ok().unwrap_or(0),
                    });
                }
            }
        }
    }

    (active_workspace, max_workspace)
}

/// Execute JavaScript in the GNOME Shell process via D-Bus.
async fn eval_shell(code: &str) -> Result<(bool, String), Box<dyn std::error::Error + Send + Sync>> {
    let connection = zbus::Connection::session().await?;
    let proxy = crate::workspace::dbus::GnomeShellEvalProxy::new(&connection).await?;
    Ok(proxy.eval(code).await?)
}

impl MessageHandler<FfiEnvelopePayload<SwitchWorkspaceMessage>> for GnomeWorkspaceService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchWorkspaceMessage>, _sender_id: &str) {
        debug!("GNOME service: queueing workspace switch to {}", message.0.workspace_id);
        let _ = self.command_sender.send(GnomeCommand::SwitchWorkspace(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<CreateWorkspaceMessage>> for GnomeWorkspaceService {
    fn handle_message(&self, message: FfiEnvelopePayload<CreateWorkspaceMessage>, _sender_id: &str) {
        debug!(
            "GNOME service: queueing workspace creation relative_to={}, position={:?}",
            message.0.relative_to, message.0.position
        );
        let _ = self.command_sender.send(GnomeCommand::CreateWorkspace(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>> for GnomeWorkspaceService {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>, _sender_id: &str) {
        debug!("GNOME service: queueing workspace snapshot request");
        let _ = self.command_sender.send(GnomeCommand::SnapshotRequest(message.0));
    }
}

impl MessageBroadcaster for GnomeWorkspaceService {}

impl PluginMetaGetter for GnomeWorkspaceService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for GnomeWorkspaceService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for GnomeWorkspaceService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            debug!("GNOME service received message: topic={}, type_id={}", envelope.topic.to_string(), envelope.type_id);
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
                    warn!("GNOME service: unknown message type");
                }
            }
        }
    }
}
