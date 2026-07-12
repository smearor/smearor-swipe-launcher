use crate::config::HyprlandServiceConfig;
use crate::monitor::MonitorEvent;
use crate::monitor::spawn_monitor_listener;
use crate::monitor::spawn_monitor_worker;
use crate::workspace::WorkspaceEvent;
use crate::workspace::spawn_workspace_listener;
use crate::workspace::spawn_workspace_worker;
use hyprland::dispatch::Dispatch;
use hyprland::dispatch::DispatchType;
use hyprland::dispatch::FirstEmpty;
use hyprland::shared::HyprData;
use hyprland::shared::HyprDataActive;
use smearor_hyprland_model::ExecDispatchMessage;
use smearor_hyprland_model::HyprlandDirection;
use smearor_hyprland_model::HyprlandDispatchActionKind;
use smearor_hyprland_model::HyprlandDispatchMessage;
use smearor_hyprland_model::HyprlandFullscreenType;
use smearor_hyprland_model::HyprlandWorkspaceIdentifierKind;
use smearor_hyprland_model::KillActiveWindowDispatchMessage;
use smearor_hyprland_model::MoveFocusDispatchMessage;
use smearor_hyprland_model::MoveToWorkspaceDispatchMessage;
use smearor_hyprland_model::ToggleFloatingDispatchMessage;
use smearor_hyprland_model::ToggleFullscreenDispatchMessage;
use smearor_hyprland_model::WorkspaceDispatchMessage;
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
use stabby::option::Option as StabbyOption;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::warn;

/// Internal union of all command types the service handles.
pub enum HyprlandCommand {
    Dispatch(HyprlandDispatchMessage),
    SwitchWorkspace(SwitchWorkspaceMessage),
    CreateWorkspace(CreateWorkspaceMessage),
    SnapshotRequest(WorkspaceSnapshotRequestMessage),
}

/// Hyprland service plugin.
pub struct HyprlandService {
    /// Plugin metadata.
    pub meta: PluginMeta,
    /// Optional core context for broadcasting messages.
    pub core_context: Option<FfiCoreContext>,
    /// Sender for commands into the async worker thread.
    pub command_sender: mpsc::UnboundedSender<HyprlandCommand>,
    /// Shared configuration for the service.
    pub config: Arc<HyprlandServiceConfig>,
}

impl HyprlandService {
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

        let (command_sender, mut command_receiver) = mpsc::unbounded_channel::<HyprlandCommand>();

        let service_config = Arc::new(service_config);
        let service = HyprlandService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            command_sender,
            config: service_config,
        };

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(error) => {
                    error!("Hyprland Service: failed to create tokio runtime: {error}");
                    return;
                }
            };

            rt.block_on(async move {
                while let Some(command) = command_receiver.recv().await {
                    match command {
                        HyprlandCommand::Dispatch(message) => {
                            handle_dispatch(message).await;
                        }
                        HyprlandCommand::SwitchWorkspace(message) => {
                            handle_switch_workspace(message).await;
                        }
                        HyprlandCommand::CreateWorkspace(message) => {
                            handle_create_workspace(message).await;
                        }
                        HyprlandCommand::SnapshotRequest(message) => {
                            handle_snapshot_request(message, core_context.clone()).await;
                        }
                    }
                }
            });
        });

        // Spawn workspace event listener and worker if workspace tracking is enabled
        if service.config.enable_workspace_tracking {
            let ws_core_context = service.core_context.clone();
            let ws_meta = service.meta.clone();
            let enable_workspace_lifecycle = service.config.enable_workspace_lifecycle;

            let (ws_sender, ws_receiver) = mpsc::unbounded_channel::<WorkspaceEvent>();
            spawn_workspace_listener(ws_sender);
            spawn_workspace_worker(ws_receiver, ws_core_context, ws_meta, enable_workspace_lifecycle);
        }

        // Spawn monitor event listener and worker if monitor events are enabled
        if service.config.enable_monitor_events {
            let mon_core_context = service.core_context.clone();
            let mon_meta = service.meta.clone();

            let (mon_sender, mon_receiver) = mpsc::unbounded_channel::<MonitorEvent>();
            spawn_monitor_listener(mon_sender);
            spawn_monitor_worker(mon_receiver, mon_core_context, mon_meta);
        }

        Ok(service)
    }
}

/// Ensures the Hyprland socket can be found by the `hyprland` crate.
///
/// The crate reads `HYPRLAND_INSTANCE_SIGNATURE` to build the socket path.
/// If the variable is missing, this function tries to find a single Hyprland
/// instance in `/tmp/hypr` and sets the variable accordingly.
pub fn ensure_hyprland_instance_signature() {
    if let Ok(instance_signature) = env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        debug!("Found HYPRLAND_INSTANCE_SIGNATURE: '{instance_signature}'");
        return;
    }

    let runtime_dir = env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/run/user/1000"));

    let hypr_dir = runtime_dir.join("hypr");

    let entries = match fs::read_dir(&hypr_dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Could not read Hyprland runtime directory '{:?}': {}", hypr_dir, e);
            return;
        }
    };

    let mut signatures: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                if let Ok(name) = entry.file_name().into_string() {
                    signatures.push(name);
                }
            }
        }
    }

    if signatures.len() > 1 {
        error!("Multiple HYPRLAND_INSTANCE_SIGNATUREs found in {:?}: {:?}", hypr_dir, signatures);
    }

    match signatures.first() {
        None => {
            error!("No HYPRLAND_INSTANCE_SIGNATURE found in {:?}", hypr_dir);
        }
        Some(signature) => {
            error!("HYPRLAND_INSTANCE_SIGNATURE not set, using detected signature: {signature}");
            unsafe {
                env::set_var("HYPRLAND_INSTANCE_SIGNATURE", signature);
            }
        }
    }
}

async fn handle_dispatch(message: HyprlandDispatchMessage) {
    ensure_hyprland_instance_signature();
    let result = match message.kind {
        HyprlandDispatchActionKind::Exec => {
            let opt = message.exec.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_exec(payload).await,
                None => Ok(()),
            }
        }
        HyprlandDispatchActionKind::KillActiveWindow => Dispatch::call_async(DispatchType::KillActiveWindow).await,
        HyprlandDispatchActionKind::MoveFocus => {
            let opt = message.move_focus.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_focus(payload).await,
                None => Ok(()),
            }
        }
        HyprlandDispatchActionKind::MoveToWorkspace => {
            let opt = message.move_to_workspace.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_to_workspace(payload).await,
                None => Ok(()),
            }
        }
        HyprlandDispatchActionKind::ToggleFloating => {
            let opt: Option<ToggleFloatingDispatchMessage> = message.toggle_floating.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(_payload) => handle_toggle_floating().await,
                None => Ok(()),
            }
        }
        HyprlandDispatchActionKind::ToggleFullscreen => {
            let opt = message.toggle_fullscreen.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_toggle_fullscreen(payload).await,
                None => Ok(()),
            }
        }
        HyprlandDispatchActionKind::Workspace => {
            let opt = message.workspace.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_workspace(payload).await,
                None => Ok(()),
            }
        }
    };

    if let Err(error) = result {
        error!("Hyprland dispatch failed: {error}");
    }
}

async fn handle_exec(payload: ExecDispatchMessage) -> hyprland::Result<()> {
    let command = payload.command;
    Dispatch::call_async(DispatchType::Exec(&command)).await
}

async fn handle_move_focus(payload: MoveFocusDispatchMessage) -> hyprland::Result<()> {
    let direction = convert_direction(payload.direction);
    Dispatch::call_async(DispatchType::MoveFocus(direction)).await
}

async fn handle_move_to_workspace(payload: MoveToWorkspaceDispatchMessage) -> hyprland::Result<()> {
    debug!(
        "Hyprland service: dispatching move to workspace: kind={:?}, id={}",
        payload.identifier.kind, payload.identifier.id
    );
    match payload.identifier.kind {
        HyprlandWorkspaceIdentifierKind::Id => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(
                hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(payload.identifier.id),
                None,
            ))
            .await
        }
        HyprlandWorkspaceIdentifierKind::Relative => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(
                hyprland::dispatch::WorkspaceIdentifierWithSpecial::Relative(payload.identifier.id),
                None,
            ))
            .await
        }
        HyprlandWorkspaceIdentifierKind::RelativeMonitor => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(
                hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitor(payload.identifier.id),
                None,
            ))
            .await
        }
        HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(
                hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitorIncludingEmpty(payload.identifier.id),
                None,
            ))
            .await
        }
        HyprlandWorkspaceIdentifierKind::RelativeOpen => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(
                hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeOpen(payload.identifier.id),
                None,
            ))
            .await
        }
        HyprlandWorkspaceIdentifierKind::Previous => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Previous, None)).await
        }
        HyprlandWorkspaceIdentifierKind::Empty => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(
                hyprland::dispatch::WorkspaceIdentifierWithSpecial::Empty(FirstEmpty {
                    on_monitor: false,
                    next: false,
                }),
                None,
            ))
            .await
        }
        HyprlandWorkspaceIdentifierKind::Name => {
            let opt: Option<stabby::string::String> = payload.identifier.name.into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str()).unwrap_or("");
            Dispatch::call_async(DispatchType::MoveToWorkspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Name(name_ref), None)).await
        }
        HyprlandWorkspaceIdentifierKind::Special => {
            let opt: Option<stabby::string::String> = payload.identifier.special_name.into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str());
            Dispatch::call_async(DispatchType::MoveToWorkspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Special(name_ref), None)).await
        }
    }
}

async fn handle_toggle_floating() -> hyprland::Result<()> {
    debug!("Hyprland service: dispatching toggle floating");
    Dispatch::call_async(DispatchType::ToggleFloating(None)).await
}

async fn handle_toggle_fullscreen(payload: ToggleFullscreenDispatchMessage) -> hyprland::Result<()> {
    let fullscreen_type = convert_fullscreen_type(payload.fullscreen_type);
    Dispatch::call_async(DispatchType::ToggleFullscreen(fullscreen_type)).await
}

async fn handle_workspace(payload: WorkspaceDispatchMessage) -> hyprland::Result<()> {
    debug!(
        "Hyprland service: dispatching workspace change: kind={:?}, id={}",
        payload.identifier.kind, payload.identifier.id
    );
    match payload.identifier.kind {
        HyprlandWorkspaceIdentifierKind::Id => {
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(payload.identifier.id))).await
        }
        HyprlandWorkspaceIdentifierKind::Relative => {
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Relative(payload.identifier.id))).await
        }
        HyprlandWorkspaceIdentifierKind::RelativeMonitor => {
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitor(
                payload.identifier.id,
            )))
            .await
        }
        HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty => {
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitorIncludingEmpty(
                payload.identifier.id,
            )))
            .await
        }
        HyprlandWorkspaceIdentifierKind::RelativeOpen => {
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeOpen(
                payload.identifier.id,
            )))
            .await
        }
        HyprlandWorkspaceIdentifierKind::Previous => {
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Previous)).await
        }
        HyprlandWorkspaceIdentifierKind::Empty => {
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Empty(FirstEmpty {
                on_monitor: false,
                next: false,
            })))
            .await
        }
        HyprlandWorkspaceIdentifierKind::Name => {
            let opt: Option<stabby::string::String> = payload.identifier.name.into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str()).unwrap_or("");
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Name(name_ref))).await
        }
        HyprlandWorkspaceIdentifierKind::Special => {
            let opt: Option<stabby::string::String> = payload.identifier.special_name.into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str());
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Special(name_ref))).await
        }
    }
}

/// Handle a compositor-unified SwitchWorkspaceMessage by translating it to a
/// Hyprland workspace dispatch.
async fn handle_switch_workspace(message: SwitchWorkspaceMessage) {
    ensure_hyprland_instance_signature();
    debug!("Hyprland service: switching to workspace {}", message.workspace_id);
    let result = Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(message.workspace_id))).await;
    if let Err(error) = result {
        error!("Hyprland service: failed to switch workspace: {error}");
    }
}

/// Handle a compositor-unified CreateWorkspaceMessage.
///
/// Hyprland supports creating workspaces by dispatching to a new workspace ID.
/// For `After`, we use `workspace +1` relative to the reference workspace.
/// For `Before`, we use `workspace -1`.
async fn handle_create_workspace(message: CreateWorkspaceMessage) {
    ensure_hyprland_instance_signature();
    let new_id = match message.position {
        WorkspaceCreatePosition::After => message.relative_to + 1,
        WorkspaceCreatePosition::Before => message.relative_to - 1,
    };
    debug!(
        "Hyprland service: creating workspace at {} (relative_to={}, position={:?})",
        new_id, message.relative_to, message.position
    );
    let result = Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(new_id))).await;
    if let Err(error) = result {
        error!("Hyprland service: failed to create workspace: {error}");
    }
}

/// Handle a WorkspaceSnapshotRequestMessage by querying Hyprland for all
/// workspaces and broadcasting a WorkspaceSnapshotMessage.
async fn handle_snapshot_request(_message: WorkspaceSnapshotRequestMessage, core_context: Option<FfiCoreContext>) {
    ensure_hyprland_instance_signature();
    debug!("Hyprland service: building workspace snapshot");

    let workspaces = match hyprland::data::Workspaces::get_async().await {
        Ok(ws) => ws,
        Err(error) => {
            error!("Hyprland service: failed to query workspaces: {error}");
            return;
        }
    };

    let active_workspace = match hyprland::data::Workspace::get_active_async().await {
        Ok(ws) => ws,
        Err(error) => {
            error!("Hyprland service: failed to query active workspace: {error}");
            return;
        }
    };

    let active_id = active_workspace.id;
    let active_monitor = active_workspace.monitor;

    let mut ws_list: Vec<WorkspaceInfo> = Vec::new();
    for ws in workspaces {
        let id = ws.id;
        let name = ws.name;
        let monitor_index = ws.monitor.parse::<u32>().ok().unwrap_or(0);
        let is_active = id == active_id;
        ws_list.push(WorkspaceInfo {
            workspace_id: id,
            workspace_name: name.into(),
            monitor_index,
            is_active,
        });
    }

    let active_monitor_index = active_monitor.parse::<u32>().ok().unwrap_or(0);

    let snapshot = WorkspaceSnapshotMessage {
        workspaces: ws_list.into_iter().collect(),
        active_workspace_id: active_id,
        active_monitor_index,
    };

    let Some(ctx) = core_context else {
        return;
    };
    let broadcaster = MessageBroadcasterInner {
        meta: PluginMeta::new("hyprland-service".to_string(), "Hyprland Service".to_string(), None),
        core_context: Some(ctx),
    };
    broadcaster.broadcast_message_to_topic(snapshot);
}

fn convert_direction(direction: HyprlandDirection) -> hyprland::dispatch::Direction {
    match direction {
        HyprlandDirection::Up => hyprland::dispatch::Direction::Up,
        HyprlandDirection::Down => hyprland::dispatch::Direction::Down,
        HyprlandDirection::Left => hyprland::dispatch::Direction::Left,
        HyprlandDirection::Right => hyprland::dispatch::Direction::Right,
    }
}

fn convert_fullscreen_type(fullscreen_type: HyprlandFullscreenType) -> hyprland::dispatch::FullscreenType {
    match fullscreen_type {
        HyprlandFullscreenType::Real => hyprland::dispatch::FullscreenType::Real,
        HyprlandFullscreenType::Maximize => hyprland::dispatch::FullscreenType::Maximize,
        HyprlandFullscreenType::NoParam => hyprland::dispatch::FullscreenType::NoParam,
    }
}

impl MessageHandler<FfiEnvelopePayload<HyprlandDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<WorkspaceDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceDispatchMessage>, _sender_id: &str) {
        debug!("Hyprland service: queueing workspace dispatch for {:?}", message.0.identifier);
        let dispatch_message = HyprlandDispatchMessage {
            kind: HyprlandDispatchActionKind::Workspace,
            exec: StabbyOption::None(),
            kill_active_window: StabbyOption::None(),
            move_focus: StabbyOption::None(),
            move_to_workspace: StabbyOption::None(),
            toggle_floating: StabbyOption::None(),
            toggle_fullscreen: StabbyOption::None(),
            workspace: StabbyOption::Some(message.0.into()),
        };
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<ExecDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<ExecDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandDispatchMessage {
            kind: HyprlandDispatchActionKind::Exec,
            exec: StabbyOption::Some(message.0.into()),
            kill_active_window: StabbyOption::None(),
            move_focus: StabbyOption::None(),
            move_to_workspace: StabbyOption::None(),
            toggle_floating: StabbyOption::None(),
            toggle_fullscreen: StabbyOption::None(),
            workspace: StabbyOption::None(),
        };
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<KillActiveWindowDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<KillActiveWindowDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandDispatchMessage {
            kind: HyprlandDispatchActionKind::KillActiveWindow,
            exec: StabbyOption::None(),
            kill_active_window: StabbyOption::Some(message.0.into()),
            move_focus: StabbyOption::None(),
            move_to_workspace: StabbyOption::None(),
            toggle_floating: StabbyOption::None(),
            toggle_fullscreen: StabbyOption::None(),
            workspace: StabbyOption::None(),
        };
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<MoveFocusDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<MoveFocusDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandDispatchMessage {
            kind: HyprlandDispatchActionKind::MoveFocus,
            exec: StabbyOption::None(),
            kill_active_window: StabbyOption::None(),
            move_focus: StabbyOption::Some(message.0.into()),
            move_to_workspace: StabbyOption::None(),
            toggle_floating: StabbyOption::None(),
            toggle_fullscreen: StabbyOption::None(),
            workspace: StabbyOption::None(),
        };
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<ToggleFullscreenDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<ToggleFullscreenDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandDispatchMessage {
            kind: HyprlandDispatchActionKind::ToggleFullscreen,
            exec: StabbyOption::None(),
            kill_active_window: StabbyOption::None(),
            move_focus: StabbyOption::None(),
            move_to_workspace: StabbyOption::None(),
            toggle_floating: StabbyOption::None(),
            toggle_fullscreen: StabbyOption::Some(message.0.into()),
            workspace: StabbyOption::None(),
        };
        let _ = self.command_sender.send(HyprlandCommand::Dispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<SwitchWorkspaceMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchWorkspaceMessage>, _sender_id: &str) {
        debug!("Hyprland service: queueing workspace switch to {}", message.0.workspace_id);
        let _ = self.command_sender.send(HyprlandCommand::SwitchWorkspace(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<CreateWorkspaceMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<CreateWorkspaceMessage>, _sender_id: &str) {
        debug!(
            "Hyprland service: queueing workspace creation relative_to={}, position={:?}",
            message.0.relative_to, message.0.position
        );
        let _ = self.command_sender.send(HyprlandCommand::CreateWorkspace(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceSnapshotRequestMessage>, _sender_id: &str) {
        debug!("Hyprland service: queueing workspace snapshot request");
        let _ = self.command_sender.send(HyprlandCommand::SnapshotRequest(message.0));
    }
}

impl MessageBroadcaster for HyprlandService {}

impl PluginMetaGetter for HyprlandService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for HyprlandService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for HyprlandService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            debug!("Hyprland service received message: topic={}, type_id={}", envelope.topic.to_string(), envelope.type_id);
            match envelope.type_id {
                id if id == FfiEnvelopePayload::<HyprlandDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<WorkspaceDispatchMessage>::TYPE_ID => {
                    debug!("WorkspaceDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<WorkspaceDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<ExecDispatchMessage>::TYPE_ID => {
                    debug!("ExecDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<ExecDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<KillActiveWindowDispatchMessage>::TYPE_ID => {
                    debug!("KillActiveWindowDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<KillActiveWindowDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<MoveFocusDispatchMessage>::TYPE_ID => {
                    debug!("MoveFocusDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<MoveFocusDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<ToggleFullscreenDispatchMessage>::TYPE_ID => {
                    debug!("ToggleFullscreenDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<ToggleFullscreenDispatchMessage>>::handle_envelope_message(self, envelope);
                }
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
                    warn!("Unknown message type");
                }
            }
        }
    }
}
