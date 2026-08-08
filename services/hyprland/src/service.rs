use crate::config::HyprlandServiceConfig;
use crate::event_listener::spawn_event_listener;
use crate::event_listener::spawn_event_worker;
use hyprland::dispatch::Dispatch;
use hyprland::dispatch::DispatchType;
use hyprland::dispatch::FirstEmpty;
use hyprland::shared::HyprData;
use hyprland::shared::HyprDataActive;
use smearor_hyprland_model::ChangeGroupActiveDispatchMessage;
use smearor_hyprland_model::ChangeSplitRatioDispatchMessage;
use smearor_hyprland_model::CloseWindowDispatchMessage;
use smearor_hyprland_model::CustomDispatchMessage;
use smearor_hyprland_model::CycleWindowDispatchMessage;
use smearor_hyprland_model::ExecDispatchMessage;
use smearor_hyprland_model::FocusMasterDispatchMessage;
use smearor_hyprland_model::FocusMonitorDispatchMessage;
use smearor_hyprland_model::FocusWindowDispatchMessage;
use smearor_hyprland_model::GlobalDispatchMessage;
use smearor_hyprland_model::HyprlandColor;
use smearor_hyprland_model::HyprlandCorner;
use smearor_hyprland_model::HyprlandCycleDirection;
use smearor_hyprland_model::HyprlandDirection;
use smearor_hyprland_model::HyprlandFocusMasterParam;
use smearor_hyprland_model::HyprlandFullscreenType;
use smearor_hyprland_model::HyprlandLockType;
use smearor_hyprland_model::HyprlandMonitorIdentifier;
use smearor_hyprland_model::HyprlandMonitorIdentifierKind;
use smearor_hyprland_model::HyprlandNotifyIcon;
use smearor_hyprland_model::HyprlandOutputBackend;
use smearor_hyprland_model::HyprlandPosition;
use smearor_hyprland_model::HyprlandPositionKind;
use smearor_hyprland_model::HyprlandPropType;
use smearor_hyprland_model::HyprlandPropTypeKind;
use smearor_hyprland_model::HyprlandStateMessage;
use smearor_hyprland_model::HyprlandStateRequestMessage;
use smearor_hyprland_model::HyprlandSwapWithMasterParam;
use smearor_hyprland_model::HyprlandSwitchXkbLayoutCmd;
use smearor_hyprland_model::HyprlandSwitchXkbLayoutCmdKind;
use smearor_hyprland_model::HyprlandSystemDispatchMessage;
use smearor_hyprland_model::HyprlandToggleDispatchMessage;
use smearor_hyprland_model::HyprlandWindowDispatchMessage;
use smearor_hyprland_model::HyprlandWindowEventData;
use smearor_hyprland_model::HyprlandWindowIdentifier;
use smearor_hyprland_model::HyprlandWindowMove;
use smearor_hyprland_model::HyprlandWindowMoveKind;
use smearor_hyprland_model::HyprlandWindowSwitchDirection;
use smearor_hyprland_model::HyprlandWorkspaceDispatchMessage;
use smearor_hyprland_model::HyprlandWorkspaceIdentifier;
use smearor_hyprland_model::HyprlandWorkspaceIdentifierKind;
use smearor_hyprland_model::HyprlandWorkspaceIdentifierWithSpecial;
use smearor_hyprland_model::HyprlandWorkspaceOptions;
use smearor_hyprland_model::KillActiveWindowDispatchMessage;
use smearor_hyprland_model::KillCommandMessage;
use smearor_hyprland_model::LockGroupsDispatchMessage;
use smearor_hyprland_model::MoveActiveDispatchMessage;
use smearor_hyprland_model::MoveCurrentWorkspaceToMonitorDispatchMessage;
use smearor_hyprland_model::MoveCursorDispatchMessage;
use smearor_hyprland_model::MoveCursorToCornerDispatchMessage;
use smearor_hyprland_model::MoveFocusDispatchMessage;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceDispatchMessage;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceSilentDispatchMessage;
use smearor_hyprland_model::MoveIntoGroupDispatchMessage;
use smearor_hyprland_model::MoveToWorkspaceDispatchMessage;
use smearor_hyprland_model::MoveToWorkspaceSilentDispatchMessage;
use smearor_hyprland_model::MoveWindowDispatchMessage;
use smearor_hyprland_model::MoveWindowPixelDispatchMessage;
use smearor_hyprland_model::NotifyCommandMessage;
use smearor_hyprland_model::OutputCreateCommandMessage;
use smearor_hyprland_model::OutputRemoveCommandMessage;
use smearor_hyprland_model::PassDispatchMessage;
use smearor_hyprland_model::PluginLoadCommandMessage;
use smearor_hyprland_model::PluginUnloadCommandMessage;
use smearor_hyprland_model::ReloadCommandMessage;
use smearor_hyprland_model::RenameWorkspaceDispatchMessage;
use smearor_hyprland_model::ResizeActiveDispatchMessage;
use smearor_hyprland_model::ResizeWindowPixelDispatchMessage;
use smearor_hyprland_model::SetCursorCommandMessage;
use smearor_hyprland_model::SetCursorDispatchMessage;
use smearor_hyprland_model::SetErrorCommandMessage;
use smearor_hyprland_model::SetPropCommandMessage;
use smearor_hyprland_model::SwapActiveWorkspacesDispatchMessage;
use smearor_hyprland_model::SwapWindowDispatchMessage;
use smearor_hyprland_model::SwapWithMasterDispatchMessage;
use smearor_hyprland_model::SwitchXkbLayoutCommandMessage;
use smearor_hyprland_model::SystemDispatchKind;
use smearor_hyprland_model::SystemDispatchOps;
use smearor_hyprland_model::ToggleDispatchKind;
use smearor_hyprland_model::ToggleDispatchOps;
use smearor_hyprland_model::ToggleDpmsDispatchMessage;
use smearor_hyprland_model::ToggleFullscreenDispatchMessage;
use smearor_hyprland_model::ToggleSpecialWorkspaceDispatchMessage;
use smearor_hyprland_model::WindowDispatchKind;
use smearor_hyprland_model::WindowDispatchOps;
use smearor_hyprland_model::WorkspaceDispatchKind;
use smearor_hyprland_model::WorkspaceDispatchMessage;
use smearor_hyprland_model::WorkspaceDispatchOps;
use smearor_hyprland_model::WorkspaceOptionDispatchMessage;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::WorkspaceCreatePosition;
use smearor_model_compositor::WorkspaceInfo;
use smearor_model_compositor::WorkspaceSnapshotMessage;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::ServicePlugin;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use stabby::option::Option as StabbyOption;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// Internal union of all command types the service handles.
pub enum HyprlandCommand {
    WindowDispatch(HyprlandWindowDispatchMessage),
    WorkspaceDispatch(HyprlandWorkspaceDispatchMessage),
    ToggleDispatch(HyprlandToggleDispatchMessage),
    SystemDispatch(HyprlandSystemDispatchMessage),
    SwitchWorkspace(SwitchWorkspaceMessage),
    CreateWorkspace(CreateWorkspaceMessage),
    SnapshotRequest(WorkspaceSnapshotRequestMessage),
    StateRequest,
    CtlKill(KillCommandMessage),
    CtlNotify(NotifyCommandMessage),
    CtlOutputCreate(OutputCreateCommandMessage),
    CtlOutputRemove(OutputRemoveCommandMessage),
    CtlPluginLoad(PluginLoadCommandMessage),
    CtlPluginUnload(PluginUnloadCommandMessage),
    CtlReload(ReloadCommandMessage),
    CtlSetCursor(SetCursorCommandMessage),
    CtlSetError(SetErrorCommandMessage),
    CtlSetProp(SetPropCommandMessage),
    CtlSwitchXkbLayout(SwitchXkbLayoutCommandMessage),
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
    /// Last known Hyprland state (updated on state requests).
    pub last_state: Arc<Mutex<Option<HyprlandStateMessage>>>,
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
        let last_state = Arc::new(Mutex::new(None));
        let service = HyprlandService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            command_sender,
            config: service_config,
            last_state,
        };
        service.register_mcp_capabilities();

        let service_meta = service.meta.clone();
        let shared_last_state = Arc::clone(&service.last_state);
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
                        HyprlandCommand::WindowDispatch(message) => {
                            handle_dispatch_window(message).await;
                        }
                        HyprlandCommand::WorkspaceDispatch(message) => {
                            handle_dispatch_workspace(message).await;
                        }
                        HyprlandCommand::ToggleDispatch(message) => {
                            handle_dispatch_toggle(message).await;
                        }
                        HyprlandCommand::SystemDispatch(message) => {
                            handle_dispatch_system(message).await;
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
                        HyprlandCommand::CtlKill(message) => {
                            handle_ctl_kill(message).await;
                        }
                        HyprlandCommand::CtlNotify(message) => {
                            handle_ctl_notify(message).await;
                        }
                        HyprlandCommand::CtlOutputCreate(message) => {
                            handle_ctl_output_create(message).await;
                        }
                        HyprlandCommand::CtlOutputRemove(message) => {
                            handle_ctl_output_remove(message).await;
                        }
                        HyprlandCommand::CtlPluginLoad(message) => {
                            handle_ctl_plugin_load(message).await;
                        }
                        HyprlandCommand::CtlPluginUnload(message) => {
                            handle_ctl_plugin_unload(message).await;
                        }
                        HyprlandCommand::CtlReload(message) => {
                            handle_ctl_reload(message).await;
                        }
                        HyprlandCommand::CtlSetCursor(message) => {
                            handle_ctl_set_cursor(message).await;
                        }
                        HyprlandCommand::CtlSetError(message) => {
                            handle_ctl_set_error(message).await;
                        }
                        HyprlandCommand::CtlSetProp(message) => {
                            handle_ctl_set_prop(message).await;
                        }
                        HyprlandCommand::CtlSwitchXkbLayout(message) => {
                            handle_ctl_switch_xkb_layout(message).await;
                        }
                        HyprlandCommand::StateRequest => {
                            handle_state_request(core_context.clone(), &service_meta, &shared_last_state).await;
                        }
                    }
                }
            });
        });

        // Spawn consolidated event listener and worker if any event tracking is enabled
        if service.config.enable_workspace_tracking || service.config.enable_monitor_events || service.config.enable_status_events {
            let ev_core_context = service.core_context.clone();
            let ev_meta = service.meta.clone();
            let enable_workspace_tracking = service.config.enable_workspace_tracking;
            let enable_monitor_events = service.config.enable_monitor_events;
            let enable_status_events = service.config.enable_status_events;
            let enable_workspace_lifecycle = service.config.enable_workspace_lifecycle;

            let (event_sender, event_receiver) = mpsc::unbounded_channel();
            spawn_event_listener(event_sender, enable_workspace_tracking, enable_monitor_events, enable_status_events);
            spawn_event_worker(event_receiver, ev_core_context, ev_meta, enable_workspace_lifecycle);
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

async fn handle_dispatch_window(message: HyprlandWindowDispatchMessage) {
    ensure_hyprland_instance_signature();
    let result = handle_window_dispatch(message.kind, message.ops).await;
    if let Err(error) = result {
        error!("Hyprland window dispatch failed: {error}");
    }
}

async fn handle_dispatch_workspace(message: HyprlandWorkspaceDispatchMessage) {
    ensure_hyprland_instance_signature();
    let result = handle_workspace_dispatch(message.kind, message.ops).await;
    if let Err(error) = result {
        error!("Hyprland workspace dispatch failed: {error}");
    }
}

async fn handle_dispatch_toggle(message: HyprlandToggleDispatchMessage) {
    ensure_hyprland_instance_signature();
    let result = handle_toggle_dispatch(message.kind, message.ops).await;
    if let Err(error) = result {
        error!("Hyprland toggle dispatch failed: {error}");
    }
}

async fn handle_dispatch_system(message: HyprlandSystemDispatchMessage) {
    ensure_hyprland_instance_signature();
    let result = handle_system_dispatch(message.kind, message.ops).await;
    if let Err(error) = result {
        error!("Hyprland system dispatch failed: {error}");
    }
}

async fn handle_window_dispatch(kind: WindowDispatchKind, ops: WindowDispatchOps) -> hyprland::Result<()> {
    match kind {
        WindowDispatchKind::CenterWindow => Dispatch::call_async(DispatchType::CenterWindow).await,
        WindowDispatchKind::ChangeGroupActive => {
            let opt = ops.change_group_active.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_change_group_active(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::ChangeSplitRatio => {
            let opt = ops.change_split_ratio.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_change_split_ratio(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::CloseWindow => {
            let opt = ops.close_window.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_close_window(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::CycleWindow => {
            let opt = ops.cycle_window.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_cycle_window(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::Exec => {
            let opt = ops.exec.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_exec(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::FocusCurrentOrLast => Dispatch::call_async(DispatchType::FocusCurrentOrLast).await,
        WindowDispatchKind::FocusMaster => {
            let opt = ops.focus_master.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_focus_master(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::FocusMonitor => {
            let opt = ops.focus_monitor.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_focus_monitor(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::FocusUrgentOrLast => Dispatch::call_async(DispatchType::FocusUrgentOrLast).await,
        WindowDispatchKind::FocusWindow => {
            let opt = ops.focus_window.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_focus_window(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::KillActiveWindow => Dispatch::call_async(DispatchType::KillActiveWindow).await,
        WindowDispatchKind::MoveActive => {
            let opt = ops.move_active.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_active(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::MoveCursor => {
            let opt = ops.move_cursor.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_cursor(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::MoveCursorToCorner => {
            let opt = ops.move_cursor_to_corner.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_cursor_to_corner(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::MoveFocus => {
            let opt = ops.move_focus.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_focus(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::MoveIntoGroup => {
            let opt = ops.move_into_group.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_into_group(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::MoveWindow => {
            let opt = ops.move_window.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_window(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::MoveWindowPixel => {
            let opt = ops.move_window_pixel.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_window_pixel(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::ResizeActive => {
            let opt = ops.resize_active.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_resize_active(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::ResizeWindowPixel => {
            let opt = ops.resize_window_pixel.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_resize_window_pixel(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::SwapWindow => {
            let opt = ops.swap_window.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_swap_window(payload).await,
                None => Ok(()),
            }
        }
        WindowDispatchKind::SwapWithMaster => {
            let opt = ops.swap_with_master.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_swap_with_master(payload).await,
                None => Ok(()),
            }
        }
    }
}

async fn handle_workspace_dispatch(kind: WorkspaceDispatchKind, ops: WorkspaceDispatchOps) -> hyprland::Result<()> {
    match kind {
        WorkspaceDispatchKind::MoveCurrentWorkspaceToMonitor => {
            let opt = ops.move_current_workspace_to_monitor.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_current_workspace_to_monitor(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::MoveFocusedWindowToWorkspace => {
            let opt = ops.move_focused_window_to_workspace.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_focused_window_to_workspace(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::MoveFocusedWindowToWorkspaceSilent => {
            let opt = ops.move_focused_window_to_workspace_silent.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_focused_window_to_workspace_silent(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::MoveToWorkspace => {
            let opt = ops.move_to_workspace.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_to_workspace(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::MoveToWorkspaceSilent => {
            let opt = ops.move_to_workspace_silent.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_move_to_workspace_silent(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::RenameWorkspace => {
            let opt = ops.rename_workspace.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_rename_workspace(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::SwapActiveWorkspaces => {
            let opt = ops.swap_active_workspaces.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_swap_active_workspaces(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::ToggleSpecialWorkspace => {
            let opt = ops.toggle_special_workspace.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_toggle_special_workspace(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::Workspace => {
            let opt = ops.workspace.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_workspace(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::WorkspaceOption => {
            let opt = ops.workspace_option.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_workspace_option(payload).await,
                None => Ok(()),
            }
        }
    }
}

async fn handle_toggle_dispatch(kind: ToggleDispatchKind, ops: ToggleDispatchOps) -> hyprland::Result<()> {
    match kind {
        ToggleDispatchKind::ToggleDpms => {
            let opt = ops.toggle_dpms.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_toggle_dpms(payload).await,
                None => Ok(()),
            }
        }
        ToggleDispatchKind::ToggleFakeFullscreen => Dispatch::call_async(DispatchType::ToggleFakeFullscreen).await,
        ToggleDispatchKind::ToggleFloating => handle_toggle_floating().await,
        ToggleDispatchKind::ToggleFullscreen => {
            let opt = ops.toggle_fullscreen.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_toggle_fullscreen(payload).await,
                None => Ok(()),
            }
        }
        ToggleDispatchKind::ToggleGroup => Dispatch::call_async(DispatchType::ToggleGroup).await,
        ToggleDispatchKind::ToggleOpaque => Dispatch::call_async(DispatchType::ToggleOpaque).await,
        ToggleDispatchKind::TogglePin => Dispatch::call_async(DispatchType::TogglePin).await,
        ToggleDispatchKind::TogglePseudo => Dispatch::call_async(DispatchType::TogglePseudo).await,
        ToggleDispatchKind::ToggleSplit => Dispatch::call_async(DispatchType::ToggleSplit).await,
    }
}

async fn handle_system_dispatch(kind: SystemDispatchKind, ops: SystemDispatchOps) -> hyprland::Result<()> {
    match kind {
        SystemDispatchKind::AddMaster => Dispatch::call_async(DispatchType::AddMaster).await,
        SystemDispatchKind::BringActiveToTop => Dispatch::call_async(DispatchType::BringActiveToTop).await,
        SystemDispatchKind::Custom => {
            let opt = ops.custom.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_custom(payload).await,
                None => Ok(()),
            }
        }
        SystemDispatchKind::Exit => Dispatch::call_async(DispatchType::Exit).await,
        SystemDispatchKind::ForceRendererReload => Dispatch::call_async(DispatchType::ForceRendererReload).await,
        SystemDispatchKind::Global => {
            let opt = ops.global.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_global(payload).await,
                None => Ok(()),
            }
        }
        SystemDispatchKind::LockGroups => {
            let opt = ops.lock_groups.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_lock_groups(payload).await,
                None => Ok(()),
            }
        }
        SystemDispatchKind::MoveOutOfGroup => Dispatch::call_async(DispatchType::MoveOutOfGroup).await,
        SystemDispatchKind::OrientationBottom => Dispatch::call_async(DispatchType::OrientationBottom).await,
        SystemDispatchKind::OrientationCenter => Dispatch::call_async(DispatchType::OrientationCenter).await,
        SystemDispatchKind::OrientationLeft => Dispatch::call_async(DispatchType::OrientationLeft).await,
        SystemDispatchKind::OrientationNext => Dispatch::call_async(DispatchType::OrientationNext).await,
        SystemDispatchKind::OrientationPrev => Dispatch::call_async(DispatchType::OrientationPrev).await,
        SystemDispatchKind::OrientationRight => Dispatch::call_async(DispatchType::OrientationRight).await,
        SystemDispatchKind::OrientationTop => Dispatch::call_async(DispatchType::OrientationTop).await,
        SystemDispatchKind::Pass => {
            let opt = ops.pass.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_pass(payload).await,
                None => Ok(()),
            }
        }
        SystemDispatchKind::RemoveMaster => Dispatch::call_async(DispatchType::RemoveMaster).await,
        SystemDispatchKind::SetCursor => {
            let opt = ops.set_cursor.match_owned(|value| Some(value.into()), || None);
            match opt {
                Some(payload) => handle_set_cursor(payload).await,
                None => Ok(()),
            }
        }
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
            let opt: Option<stabby::string::String> = payload.identifier.name.clone().into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str()).unwrap_or("");
            Dispatch::call_async(DispatchType::MoveToWorkspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Name(name_ref), None)).await
        }
        HyprlandWorkspaceIdentifierKind::Special => {
            let opt: Option<stabby::string::String> = payload.identifier.special_name.clone().into();
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
            let opt: Option<stabby::string::String> = payload.identifier.name.clone().into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str()).unwrap_or("");
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Name(name_ref))).await
        }
        HyprlandWorkspaceIdentifierKind::Special => {
            let opt: Option<stabby::string::String> = payload.identifier.special_name.clone().into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str());
            Dispatch::call_async(DispatchType::Workspace(hyprland::dispatch::WorkspaceIdentifierWithSpecial::Special(name_ref))).await
        }
    }
}

async fn handle_change_group_active(payload: ChangeGroupActiveDispatchMessage) -> hyprland::Result<()> {
    let direction = convert_window_switch_direction(payload.direction);
    Dispatch::call_async(DispatchType::ChangeGroupActive(direction)).await
}

async fn handle_change_split_ratio(payload: ChangeSplitRatioDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ChangeSplitRatio(hyprland::dispatch::FloatValue::Exact(payload.ratio))).await
}

async fn handle_close_window(payload: CloseWindowDispatchMessage) -> hyprland::Result<()> {
    let win_id = convert_window_identifier(&payload.window_identifier);
    Dispatch::call_async(DispatchType::CloseWindow(win_id.as_ref())).await
}

async fn handle_custom(payload: CustomDispatchMessage) -> hyprland::Result<()> {
    let name = payload.name;
    let value = payload.value;
    Dispatch::call_async(DispatchType::Custom(&name, &value)).await
}

async fn handle_cycle_window(payload: CycleWindowDispatchMessage) -> hyprland::Result<()> {
    let direction = convert_cycle_direction(payload.cycle_direction);
    Dispatch::call_async(DispatchType::CycleWindow(direction)).await
}

async fn handle_focus_master(payload: FocusMasterDispatchMessage) -> hyprland::Result<()> {
    let param = convert_focus_master_param(payload.param);
    Dispatch::call_async(DispatchType::FocusMaster(param)).await
}

async fn handle_focus_monitor(payload: FocusMonitorDispatchMessage) -> hyprland::Result<()> {
    let name_opt: Option<stabby::string::String> = payload.monitor_identifier.name.clone().into();
    let name_string = name_opt.map(|n| n.to_string());
    let name_ref = name_string.as_ref().map(|n| n.as_str());
    let monitor = convert_monitor_identifier(&payload.monitor_identifier, name_ref);
    Dispatch::call_async(DispatchType::FocusMonitor(monitor)).await
}

async fn handle_focus_window(payload: FocusWindowDispatchMessage) -> hyprland::Result<()> {
    let win_id = convert_window_identifier(&payload.window_identifier);
    Dispatch::call_async(DispatchType::FocusWindow(win_id.as_ref())).await
}

async fn handle_global(payload: GlobalDispatchMessage) -> hyprland::Result<()> {
    let key = payload.key;
    Dispatch::call_async(DispatchType::Global(&key)).await
}

async fn handle_lock_groups(payload: LockGroupsDispatchMessage) -> hyprland::Result<()> {
    let lock_type = convert_lock_type(payload.lock_type);
    Dispatch::call_async(DispatchType::LockGroups(lock_type)).await
}

async fn handle_move_active(payload: MoveActiveDispatchMessage) -> hyprland::Result<()> {
    let position = convert_position(payload.position);
    Dispatch::call_async(DispatchType::MoveActive(position)).await
}

async fn handle_move_cursor(payload: MoveCursorDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::MoveCursor(payload.x, payload.y)).await
}

async fn handle_move_cursor_to_corner(payload: MoveCursorToCornerDispatchMessage) -> hyprland::Result<()> {
    let corner = convert_corner(payload.corner);
    Dispatch::call_async(DispatchType::MoveCursorToCorner(corner)).await
}

async fn handle_move_current_workspace_to_monitor(payload: MoveCurrentWorkspaceToMonitorDispatchMessage) -> hyprland::Result<()> {
    let name_opt: Option<stabby::string::String> = payload.monitor_identifier.name.clone().into();
    let name_string = name_opt.map(|n| n.to_string());
    let name_ref = name_string.as_ref().map(|n| n.as_str());
    let monitor = convert_monitor_identifier(&payload.monitor_identifier, name_ref);
    Dispatch::call_async(DispatchType::MoveCurrentWorkspaceToMonitor(monitor)).await
}

async fn handle_move_focused_window_to_workspace(payload: MoveFocusedWindowToWorkspaceDispatchMessage) -> hyprland::Result<()> {
    let ws_id = convert_workspace_identifier(&payload.identifier);
    Dispatch::call_async(DispatchType::MoveToWorkspace(ws_id.as_ref(), None)).await
}

async fn handle_move_focused_window_to_workspace_silent(payload: MoveFocusedWindowToWorkspaceSilentDispatchMessage) -> hyprland::Result<()> {
    let ws_id = convert_workspace_identifier(&payload.identifier);
    Dispatch::call_async(DispatchType::MoveToWorkspaceSilent(ws_id.as_ref(), None)).await
}

async fn handle_move_into_group(payload: MoveIntoGroupDispatchMessage) -> hyprland::Result<()> {
    let direction = convert_direction(payload.direction);
    Dispatch::call_async(DispatchType::MoveIntoGroup(direction)).await
}

async fn handle_move_to_workspace_silent(payload: MoveToWorkspaceSilentDispatchMessage) -> hyprland::Result<()> {
    let ws_id = convert_workspace_identifier_with_special(&payload.identifier);
    let win_id = convert_window_identifier_opt(&payload.window_identifier);
    let win_id = win_id.as_ref().map(|w| w.as_ref());
    Dispatch::call_async(DispatchType::MoveToWorkspaceSilent(ws_id.as_ref(), win_id)).await
}

async fn handle_move_window(payload: MoveWindowDispatchMessage) -> hyprland::Result<()> {
    let window_move = convert_window_move(&payload.window_move);
    Dispatch::call_async(DispatchType::MoveWindow(window_move.as_ref())).await
}

async fn handle_move_window_pixel(payload: MoveWindowPixelDispatchMessage) -> hyprland::Result<()> {
    let position = convert_position(payload.position);
    let win_id = convert_window_identifier(&payload.window_identifier);
    Dispatch::call_async(DispatchType::MoveWindowPixel(position, win_id.as_ref())).await
}

async fn handle_pass(payload: PassDispatchMessage) -> hyprland::Result<()> {
    let win_id = convert_window_identifier(&payload.window_identifier);
    Dispatch::call_async(DispatchType::Pass(win_id.as_ref())).await
}

async fn handle_rename_workspace(payload: RenameWorkspaceDispatchMessage) -> hyprland::Result<()> {
    let name_ref = payload.new_name.as_ref().map(|n| n.as_str());
    Dispatch::call_async(DispatchType::RenameWorkspace(payload.workspace_id, name_ref)).await
}

async fn handle_resize_active(payload: ResizeActiveDispatchMessage) -> hyprland::Result<()> {
    let position = convert_position(payload.position);
    Dispatch::call_async(DispatchType::ResizeActive(position)).await
}

async fn handle_resize_window_pixel(payload: ResizeWindowPixelDispatchMessage) -> hyprland::Result<()> {
    let position = convert_position(payload.position);
    let win_id = convert_window_identifier(&payload.window_identifier);
    Dispatch::call_async(DispatchType::ResizeWindowPixel(position, win_id.as_ref())).await
}

async fn handle_set_cursor(payload: SetCursorDispatchMessage) -> hyprland::Result<()> {
    let theme = payload.theme;
    Dispatch::call_async(DispatchType::SetCursor(&theme, payload.size)).await
}

async fn handle_swap_active_workspaces(payload: SwapActiveWorkspacesDispatchMessage) -> hyprland::Result<()> {
    let name_a: Option<stabby::string::String> = payload.monitor_a.name.clone().into();
    let name_a_string = name_a.map(|n| n.to_string());
    let name_a_ref = name_a_string.as_ref().map(|n| n.as_str());
    let monitor_a = convert_monitor_identifier(&payload.monitor_a, name_a_ref);
    let name_b: Option<stabby::string::String> = payload.monitor_b.name.clone().into();
    let name_b_string = name_b.map(|n| n.to_string());
    let name_b_ref = name_b_string.as_ref().map(|n| n.as_str());
    let monitor_b = convert_monitor_identifier(&payload.monitor_b, name_b_ref);
    Dispatch::call_async(DispatchType::SwapActiveWorkspaces(monitor_a, monitor_b)).await
}

async fn handle_swap_window(payload: SwapWindowDispatchMessage) -> hyprland::Result<()> {
    let direction = convert_cycle_direction(payload.cycle_direction);
    Dispatch::call_async(DispatchType::SwapNext(direction)).await
}

async fn handle_swap_with_master(payload: SwapWithMasterDispatchMessage) -> hyprland::Result<()> {
    let param = convert_swap_with_master_param(payload.param);
    Dispatch::call_async(DispatchType::SwapWithMaster(param)).await
}

async fn handle_toggle_dpms(payload: ToggleDpmsDispatchMessage) -> hyprland::Result<()> {
    let name_ref = payload.name.as_ref().map(|n| n.as_str());
    Dispatch::call_async(DispatchType::ToggleDPMS(payload.on, name_ref)).await
}

async fn handle_toggle_special_workspace(payload: ToggleSpecialWorkspaceDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ToggleSpecialWorkspace(payload.workspace_name)).await
}

async fn handle_workspace_option(payload: WorkspaceOptionDispatchMessage) -> hyprland::Result<()> {
    let opt = convert_workspace_options(payload.option);
    Dispatch::call_async(DispatchType::WorkspaceOption(opt)).await
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

/// Handle a `HyprlandStateRequestMessage` by querying the current Hyprland state
/// and broadcasting a `HyprlandStateMessage`.
///
/// Synchronous IPC calls are wrapped in `tokio::task::spawn_blocking` to prevent
/// blocking the async event worker when the compositor is under load.
async fn handle_state_request(core_context: Option<FfiCoreContext>, meta: &PluginMeta, last_state: &Arc<Mutex<Option<HyprlandStateMessage>>>) {
    ensure_hyprland_instance_signature();

    let state = tokio::task::spawn_blocking(|| {
        let active_window = hyprland::data::Clients::get()
            .ok()
            .and_then(|clients| clients.into_iter().find(|c| c.focus_history_id == 0))
            .map(|c| HyprlandWindowEventData {
                window_class: c.class.clone().into(),
                window_title: c.title.clone().into(),
                window_address: c.address.to_string().into(),
                workspace_id: c.workspace.id,
            });

        let is_fullscreen = active_window
            .as_ref()
            .is_some_and(|_w| hyprland::data::Workspace::get_active().ok().map(|ws| ws.fullscreen).unwrap_or(false));

        let keyboard_layout = hyprland::data::Devices::get()
            .ok()
            .and_then(|devices| devices.keyboards.first().map(|k| k.active_keymap.clone().into()));

        let sub_map = String::new().into();

        let ignore_group_lock = false;
        let groups_locked = false;

        HyprlandStateMessage {
            active_window: active_window.into(),
            is_fullscreen,
            keyboard_layout: keyboard_layout.into(),
            sub_map,
            ignore_group_lock,
            groups_locked,
        }
    })
    .await
    .unwrap_or_default();

    if let Ok(mut guard) = last_state.lock() {
        *guard = Some(state.clone());
    }

    let Some(ctx) = core_context else {
        return;
    };
    let broadcaster = MessageBroadcasterInner {
        meta: meta.clone(),
        core_context: Some(ctx),
    };
    broadcaster.broadcast_message_to_topic(state);
}

impl MessageHandler<FfiEnvelopePayload<HyprlandStateRequestMessage>> for HyprlandService {
    fn handle_message(&self, _message: FfiEnvelopePayload<HyprlandStateRequestMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::StateRequest);
    }
}

impl HyprlandService {
    pub(crate) fn status_snapshot(&self) -> Option<HyprlandStateMessage> {
        self.last_state.lock().ok().and_then(|s| s.clone())
    }
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

fn convert_cycle_direction(dir: HyprlandCycleDirection) -> hyprland::dispatch::CycleDirection {
    match dir {
        HyprlandCycleDirection::Next => hyprland::dispatch::CycleDirection::Next,
        HyprlandCycleDirection::Previous => hyprland::dispatch::CycleDirection::Previous,
    }
}

fn convert_window_switch_direction(dir: HyprlandWindowSwitchDirection) -> hyprland::dispatch::WindowSwitchDirection {
    match dir {
        HyprlandWindowSwitchDirection::Back => hyprland::dispatch::WindowSwitchDirection::Back,
        HyprlandWindowSwitchDirection::Forward => hyprland::dispatch::WindowSwitchDirection::Forward,
    }
}

fn convert_corner(corner: HyprlandCorner) -> hyprland::dispatch::Corner {
    match corner {
        HyprlandCorner::BottomLeft => hyprland::dispatch::Corner::BottomLeft,
        HyprlandCorner::BottomRight => hyprland::dispatch::Corner::BottomRight,
        HyprlandCorner::TopRight => hyprland::dispatch::Corner::TopRight,
        HyprlandCorner::TopLeft => hyprland::dispatch::Corner::TopLeft,
    }
}

fn convert_workspace_options(opt: HyprlandWorkspaceOptions) -> hyprland::dispatch::WorkspaceOptions {
    match opt {
        HyprlandWorkspaceOptions::AllPseudo => hyprland::dispatch::WorkspaceOptions::AllPseudo,
        HyprlandWorkspaceOptions::AllFloat => hyprland::dispatch::WorkspaceOptions::AllFloat,
    }
}

fn convert_lock_type(lt: HyprlandLockType) -> hyprland::dispatch::LockType {
    match lt {
        HyprlandLockType::Lock => hyprland::dispatch::LockType::Lock,
        HyprlandLockType::Unlock => hyprland::dispatch::LockType::Unlock,
        HyprlandLockType::ToggleLock => hyprland::dispatch::LockType::ToggleLock,
    }
}

fn convert_swap_with_master_param(param: HyprlandSwapWithMasterParam) -> hyprland::dispatch::SwapWithMasterParam {
    match param {
        HyprlandSwapWithMasterParam::Master => hyprland::dispatch::SwapWithMasterParam::Master,
        HyprlandSwapWithMasterParam::Child => hyprland::dispatch::SwapWithMasterParam::Child,
        HyprlandSwapWithMasterParam::Auto => hyprland::dispatch::SwapWithMasterParam::Auto,
    }
}

fn convert_focus_master_param(param: HyprlandFocusMasterParam) -> hyprland::dispatch::FocusMasterParam {
    match param {
        HyprlandFocusMasterParam::Master => hyprland::dispatch::FocusMasterParam::Master,
        HyprlandFocusMasterParam::Auto => hyprland::dispatch::FocusMasterParam::Auto,
    }
}

fn convert_position(pos: HyprlandPosition) -> hyprland::dispatch::Position {
    match pos.kind {
        HyprlandPositionKind::Delta => hyprland::dispatch::Position::Delta(pos.x, pos.y),
        HyprlandPositionKind::Exact => hyprland::dispatch::Position::Exact(pos.x, pos.y),
    }
}

fn convert_monitor_identifier<'a>(id: &'a HyprlandMonitorIdentifier, name: Option<&'a str>) -> hyprland::dispatch::MonitorIdentifier<'a> {
    match id.kind {
        HyprlandMonitorIdentifierKind::Current => hyprland::dispatch::MonitorIdentifier::Current,
        HyprlandMonitorIdentifierKind::Direction => hyprland::dispatch::MonitorIdentifier::Direction(convert_direction(id.direction)),
        HyprlandMonitorIdentifierKind::Id => hyprland::dispatch::MonitorIdentifier::Id(id.id as i128),
        HyprlandMonitorIdentifierKind::Name => hyprland::dispatch::MonitorIdentifier::Name(name.unwrap_or("")),
        HyprlandMonitorIdentifierKind::Relative => hyprland::dispatch::MonitorIdentifier::Relative(id.relative),
    }
}

struct OwnedWindowIdentifier {
    process_id: u32,
    address: Option<String>,
    class_regex: Option<String>,
    title: Option<String>,
    kind: OwnedWindowIdentifierKind,
}

#[derive(Clone, Copy)]
enum OwnedWindowIdentifierKind {
    ProcessId,
    Address,
    ClassRegularExpression,
    Title,
}

impl OwnedWindowIdentifier {
    fn as_ref(&self) -> hyprland::dispatch::WindowIdentifier<'_> {
        match self.kind {
            OwnedWindowIdentifierKind::ProcessId => hyprland::dispatch::WindowIdentifier::ProcessId(self.process_id),
            OwnedWindowIdentifierKind::Address => {
                hyprland::dispatch::WindowIdentifier::Address(hyprland::shared::Address::new(self.address.as_deref().unwrap_or("")))
            }
            OwnedWindowIdentifierKind::ClassRegularExpression => {
                hyprland::dispatch::WindowIdentifier::ClassRegularExpression(self.class_regex.as_deref().unwrap_or(""))
            }
            OwnedWindowIdentifierKind::Title => hyprland::dispatch::WindowIdentifier::Title(self.title.as_deref().unwrap_or("")),
        }
    }
}

fn convert_window_identifier(id: &HyprlandWindowIdentifier) -> OwnedWindowIdentifier {
    id.match_ref(
        |pid| OwnedWindowIdentifier {
            process_id: *pid,
            address: None,
            class_regex: None,
            title: None,
            kind: OwnedWindowIdentifierKind::ProcessId,
        },
        |addr| OwnedWindowIdentifier {
            process_id: 0,
            address: Some(addr.to_string()),
            class_regex: None,
            title: None,
            kind: OwnedWindowIdentifierKind::Address,
        },
        |s| OwnedWindowIdentifier {
            process_id: 0,
            address: None,
            class_regex: Some(s.to_string()),
            title: None,
            kind: OwnedWindowIdentifierKind::ClassRegularExpression,
        },
        |s| OwnedWindowIdentifier {
            process_id: 0,
            address: None,
            class_regex: None,
            title: Some(s.to_string()),
            kind: OwnedWindowIdentifierKind::Title,
        },
    )
}

fn convert_window_identifier_opt(id: &Option<HyprlandWindowIdentifier>) -> Option<OwnedWindowIdentifier> {
    id.as_ref().map(convert_window_identifier)
}

struct OwnedWorkspaceIdentifierWithSpecial {
    id: i32,
    name: Option<String>,
    special_name: Option<String>,
    kind: HyprlandWorkspaceIdentifierKind,
}

impl OwnedWorkspaceIdentifierWithSpecial {
    fn as_ref(&self) -> hyprland::dispatch::WorkspaceIdentifierWithSpecial<'_> {
        match self.kind {
            HyprlandWorkspaceIdentifierKind::Id => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Id(self.id),
            HyprlandWorkspaceIdentifierKind::Relative => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Relative(self.id),
            HyprlandWorkspaceIdentifierKind::RelativeMonitor => hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitor(self.id),
            HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty => {
                hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeMonitorIncludingEmpty(self.id)
            }
            HyprlandWorkspaceIdentifierKind::RelativeOpen => hyprland::dispatch::WorkspaceIdentifierWithSpecial::RelativeOpen(self.id),
            HyprlandWorkspaceIdentifierKind::Previous => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Previous,
            HyprlandWorkspaceIdentifierKind::Empty => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Empty(FirstEmpty {
                on_monitor: false,
                next: false,
            }),
            HyprlandWorkspaceIdentifierKind::Name => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Name(self.name.as_deref().unwrap_or("")),
            HyprlandWorkspaceIdentifierKind::Special => hyprland::dispatch::WorkspaceIdentifierWithSpecial::Special(self.special_name.as_deref()),
        }
    }
}

fn convert_workspace_identifier_with_special(id: &HyprlandWorkspaceIdentifierWithSpecial) -> OwnedWorkspaceIdentifierWithSpecial {
    let name: Option<stabby::string::String> = id.name.clone().into();
    let special_name: Option<stabby::string::String> = id.special_name.clone().into();
    OwnedWorkspaceIdentifierWithSpecial {
        id: id.id,
        name: name.map(|n| n.to_string()),
        special_name: special_name.map(|n| n.to_string()),
        kind: id.kind,
    }
}

fn convert_workspace_identifier(id: &HyprlandWorkspaceIdentifier) -> OwnedWorkspaceIdentifierWithSpecial {
    id.match_ref(
        || OwnedWorkspaceIdentifierWithSpecial {
            id: 0,
            name: None,
            special_name: None,
            kind: HyprlandWorkspaceIdentifierKind::Previous,
        },
        || OwnedWorkspaceIdentifierWithSpecial {
            id: 0,
            name: None,
            special_name: None,
            kind: HyprlandWorkspaceIdentifierKind::Empty,
        },
        |i| OwnedWorkspaceIdentifierWithSpecial {
            id: *i,
            name: None,
            special_name: None,
            kind: HyprlandWorkspaceIdentifierKind::Id,
        },
        |i| OwnedWorkspaceIdentifierWithSpecial {
            id: *i,
            name: None,
            special_name: None,
            kind: HyprlandWorkspaceIdentifierKind::Relative,
        },
        |i| OwnedWorkspaceIdentifierWithSpecial {
            id: *i,
            name: None,
            special_name: None,
            kind: HyprlandWorkspaceIdentifierKind::RelativeMonitor,
        },
        |i| OwnedWorkspaceIdentifierWithSpecial {
            id: *i,
            name: None,
            special_name: None,
            kind: HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty,
        },
        |i| OwnedWorkspaceIdentifierWithSpecial {
            id: *i,
            name: None,
            special_name: None,
            kind: HyprlandWorkspaceIdentifierKind::RelativeOpen,
        },
        |s| OwnedWorkspaceIdentifierWithSpecial {
            id: 0,
            name: Some(s.to_string()),
            special_name: None,
            kind: HyprlandWorkspaceIdentifierKind::Name,
        },
    )
}

struct OwnedWindowMove {
    monitor_name: Option<String>,
    direction: HyprlandDirection,
    kind: HyprlandWindowMoveKind,
    monitor: HyprlandMonitorIdentifier,
}

impl OwnedWindowMove {
    fn as_ref(&self) -> hyprland::dispatch::WindowMove<'_> {
        match self.kind {
            HyprlandWindowMoveKind::Direction => hyprland::dispatch::WindowMove::Direction(convert_direction(self.direction)),
            HyprlandWindowMoveKind::Monitor => {
                let name_ref = self.monitor_name.as_ref().map(|n| n.as_str());
                hyprland::dispatch::WindowMove::Monitor(convert_monitor_identifier(&self.monitor, name_ref))
            }
        }
    }
}

fn convert_window_move(wm: &HyprlandWindowMove) -> OwnedWindowMove {
    let name: Option<stabby::string::String> = wm.monitor.name.clone().into();
    OwnedWindowMove {
        monitor_name: name.map(|n| n.to_string()),
        direction: wm.direction,
        kind: wm.kind,
        monitor: wm.monitor.clone(),
    }
}

fn convert_notify_icon(icon: HyprlandNotifyIcon) -> hyprland::ctl::notify::Icon {
    match icon {
        HyprlandNotifyIcon::Warning => hyprland::ctl::notify::Icon::Warning,
        HyprlandNotifyIcon::Info => hyprland::ctl::notify::Icon::Info,
        HyprlandNotifyIcon::Hint => hyprland::ctl::notify::Icon::Hint,
        HyprlandNotifyIcon::Error => hyprland::ctl::notify::Icon::Error,
        HyprlandNotifyIcon::Confused => hyprland::ctl::notify::Icon::Confused,
        HyprlandNotifyIcon::Ok => hyprland::ctl::notify::Icon::Ok,
        HyprlandNotifyIcon::NoIcon => hyprland::ctl::notify::Icon::NoIcon,
    }
}

fn convert_color(color: HyprlandColor) -> hyprland::ctl::Color {
    hyprland::ctl::Color::new(color.red, color.green, color.blue, color.alpha)
}

fn convert_output_backend(backend: HyprlandOutputBackend) -> hyprland::ctl::output::OutputBackends {
    match backend {
        HyprlandOutputBackend::Wayland => hyprland::ctl::output::OutputBackends::Wayland,
        HyprlandOutputBackend::X11 => hyprland::ctl::output::OutputBackends::X11,
        HyprlandOutputBackend::Headless => hyprland::ctl::output::OutputBackends::Headless,
        HyprlandOutputBackend::Auto => hyprland::ctl::output::OutputBackends::Auto,
    }
}

fn convert_switch_xkb_layout_cmd(cmd: HyprlandSwitchXkbLayoutCmd) -> hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes {
    match cmd.kind {
        HyprlandSwitchXkbLayoutCmdKind::Next => hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Next,
        HyprlandSwitchXkbLayoutCmdKind::Previous => hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Previous,
        HyprlandSwitchXkbLayoutCmdKind::Id => hyprland::ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes::Id(cmd.id),
    }
}

fn convert_prop_type(prop: HyprlandPropType) -> hyprland::ctl::set_prop::PropType {
    let animation_style: Option<stabby::string::String> = prop.animation_style.into();
    let animation_style_string = animation_style.map(|s| s.to_string());
    match prop.kind {
        HyprlandPropTypeKind::AnimationStyle => hyprland::ctl::set_prop::PropType::AnimationStyle(animation_style_string.unwrap_or_default()),
        HyprlandPropTypeKind::Rounding => hyprland::ctl::set_prop::PropType::Rounding(prop.rounding, prop.locked),
        HyprlandPropTypeKind::ForceNoBlur => hyprland::ctl::set_prop::PropType::ForceNoBlur(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceOpaque => hyprland::ctl::set_prop::PropType::ForceOpaque(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceOpaqueOverriden => hyprland::ctl::set_prop::PropType::ForceOpaqueOverriden(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceAllowsInput => hyprland::ctl::set_prop::PropType::ForceAllowsInput(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceNoAnims => hyprland::ctl::set_prop::PropType::ForceNoAnims(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceNoBorder => hyprland::ctl::set_prop::PropType::ForceNoBorder(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::ForceNoShadow => hyprland::ctl::set_prop::PropType::ForceNoShadow(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::WindowDanceCompat => hyprland::ctl::set_prop::PropType::WindowDanceCompat(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::NoMaxSize => hyprland::ctl::set_prop::PropType::NoMaxSize(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::DimAround => hyprland::ctl::set_prop::PropType::DimAround(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::AlphaOverride => hyprland::ctl::set_prop::PropType::AlphaOverride(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::Alpha => hyprland::ctl::set_prop::PropType::Alpha(prop.value_float, prop.locked),
        HyprlandPropTypeKind::AlphaInactiveOverride => hyprland::ctl::set_prop::PropType::AlphaInactiveOverride(prop.value_bool, prop.locked),
        HyprlandPropTypeKind::AlphaInactive => hyprland::ctl::set_prop::PropType::AlphaInactive(prop.value_float, prop.locked),
        HyprlandPropTypeKind::ActiveBorderColor => hyprland::ctl::set_prop::PropType::ActiveBorderColor(convert_color(prop.color), prop.locked),
        HyprlandPropTypeKind::InactiveBorderColor => hyprland::ctl::set_prop::PropType::InactiveBorderColor(convert_color(prop.color), prop.locked),
    }
}

async fn handle_ctl_kill(_message: KillCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::ctl::kill::call_async().await {
        error!("Hyprland kill failed: {error}");
    }
}

async fn handle_ctl_notify(message: NotifyCommandMessage) {
    ensure_hyprland_instance_signature();
    let icon = convert_notify_icon(message.icon);
    let color = convert_color(message.color);
    let duration = std::time::Duration::from_millis(message.time_ms as u64);
    if let Err(error) = hyprland::ctl::notify::call_async(icon, duration, color, message.message).await {
        error!("Hyprland notify failed: {error}");
    }
}

async fn handle_ctl_output_create(message: OutputCreateCommandMessage) {
    ensure_hyprland_instance_signature();
    let backend = convert_output_backend(message.backend);
    if let Err(error) = hyprland::ctl::output::create_async(backend, None).await {
        error!("Hyprland output create failed: {error}");
    }
}

async fn handle_ctl_output_remove(message: OutputRemoveCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::ctl::output::remove_async(&message.name).await {
        error!("Hyprland output remove failed: {error}");
    }
}

async fn handle_ctl_plugin_load(message: PluginLoadCommandMessage) {
    ensure_hyprland_instance_signature();
    let path = std::path::Path::new(&message.path);
    if let Err(error) = hyprland::ctl::plugin::load_async(path).await {
        error!("Hyprland plugin load failed: {error}");
    }
}

async fn handle_ctl_plugin_unload(message: PluginUnloadCommandMessage) {
    ensure_hyprland_instance_signature();
    let path = std::path::Path::new(&message.name);
    if let Err(error) = hyprland::ctl::plugin::unload_async(path).await {
        error!("Hyprland plugin unload failed: {error}");
    }
}

async fn handle_ctl_reload(_message: ReloadCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::ctl::reload::call_async().await {
        error!("Hyprland reload failed: {error}");
    }
}

async fn handle_ctl_set_cursor(message: SetCursorCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::ctl::set_cursor::call_async(&message.theme, message.size).await {
        error!("Hyprland set cursor failed: {error}");
    }
}

async fn handle_ctl_set_error(message: SetErrorCommandMessage) {
    ensure_hyprland_instance_signature();
    let color = convert_color(message.color);
    if let Err(error) = hyprland::ctl::set_error::call_async(color, message.message).await {
        error!("Hyprland set error failed: {error}");
    }
}

async fn handle_ctl_set_prop(message: SetPropCommandMessage) {
    ensure_hyprland_instance_signature();
    let prop = convert_prop_type(message.prop);
    if let Err(error) = hyprland::ctl::set_prop::call_async(message.identifier, prop, message.lock).await {
        error!("Hyprland set prop failed: {error}");
    }
}

async fn handle_ctl_switch_xkb_layout(message: SwitchXkbLayoutCommandMessage) {
    ensure_hyprland_instance_signature();
    let cmd = convert_switch_xkb_layout_cmd(message.cmd);
    if let Err(error) = hyprland::ctl::switch_xkb_layout::call(&message.device, cmd) {
        error!("Hyprland switch xkb layout failed: {error}");
    }
}

impl MessageHandler<FfiEnvelopePayload<HyprlandWindowDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandWindowDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<HyprlandWorkspaceDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandWorkspaceDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<HyprlandToggleDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandToggleDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<HyprlandSystemDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<HyprlandSystemDispatchMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::SystemDispatch(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<WorkspaceDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<WorkspaceDispatchMessage>, _sender_id: &str) {
        debug!("Hyprland service: queueing workspace dispatch for {:?}", message.0.identifier);
        let dispatch_message = HyprlandWorkspaceDispatchMessage {
            kind: WorkspaceDispatchKind::Workspace,
            ops: WorkspaceDispatchOps {
                workspace: StabbyOption::Some(message.0.into()),
                ..WorkspaceDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::WorkspaceDispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<ExecDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<ExecDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandWindowDispatchMessage {
            kind: WindowDispatchKind::Exec,
            ops: WindowDispatchOps {
                exec: StabbyOption::Some(message.0.into()),
                ..WindowDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<KillActiveWindowDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<KillActiveWindowDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandWindowDispatchMessage {
            kind: WindowDispatchKind::KillActiveWindow,
            ops: WindowDispatchOps {
                kill_active_window: StabbyOption::Some(message.0.into()),
                ..WindowDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<MoveFocusDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<MoveFocusDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandWindowDispatchMessage {
            kind: WindowDispatchKind::MoveFocus,
            ops: WindowDispatchOps {
                move_focus: StabbyOption::Some(message.0.into()),
                ..WindowDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::WindowDispatch(dispatch_message));
    }
}

impl MessageHandler<FfiEnvelopePayload<ToggleFullscreenDispatchMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<ToggleFullscreenDispatchMessage>, _sender_id: &str) {
        let dispatch_message = HyprlandToggleDispatchMessage {
            kind: ToggleDispatchKind::ToggleFullscreen,
            ops: ToggleDispatchOps {
                toggle_fullscreen: StabbyOption::Some(message.0.into()),
                ..ToggleDispatchOps::default()
            },
        };
        let _ = self.command_sender.send(HyprlandCommand::ToggleDispatch(dispatch_message));
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

impl MessageHandler<FfiEnvelopePayload<KillCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<KillCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlKill(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<NotifyCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<NotifyCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlNotify(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<OutputCreateCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<OutputCreateCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlOutputCreate(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<OutputRemoveCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<OutputRemoveCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlOutputRemove(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<PluginLoadCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<PluginLoadCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlPluginLoad(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<PluginUnloadCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<PluginUnloadCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlPluginUnload(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<ReloadCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<ReloadCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlReload(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<SetCursorCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SetCursorCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSetCursor(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<SetErrorCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SetErrorCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSetError(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<SetPropCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SetPropCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSetProp(message.0));
    }
}

impl MessageHandler<FfiEnvelopePayload<SwitchXkbLayoutCommandMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<SwitchXkbLayoutCommandMessage>, _sender_id: &str) {
        let _ = self.command_sender.send(HyprlandCommand::CtlSwitchXkbLayout(message.0));
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

impl ServicePlugin for HyprlandService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            trace!("Hyprland service received message: topic={}, type_id={}", envelope.topic.to_string(), envelope.type_id);
            match envelope.type_id {
                id if id == FfiEnvelopePayload::<HyprlandWindowDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandWindowDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandWindowDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<HyprlandWorkspaceDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandWorkspaceDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandWorkspaceDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<HyprlandToggleDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandToggleDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandToggleDispatchMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<HyprlandSystemDispatchMessage>::TYPE_ID => {
                    debug!("HyprlandSystemDispatchMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandSystemDispatchMessage>>::handle_envelope_message(self, envelope);
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
                id if id == FfiEnvelopePayload::<KillCommandMessage>::TYPE_ID => {
                    debug!("KillCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<KillCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<NotifyCommandMessage>::TYPE_ID => {
                    debug!("NotifyCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<NotifyCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<OutputCreateCommandMessage>::TYPE_ID => {
                    debug!("OutputCreateCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<OutputCreateCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<OutputRemoveCommandMessage>::TYPE_ID => {
                    debug!("OutputRemoveCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<OutputRemoveCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<PluginLoadCommandMessage>::TYPE_ID => {
                    debug!("PluginLoadCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<PluginLoadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<PluginUnloadCommandMessage>::TYPE_ID => {
                    debug!("PluginUnloadCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<PluginUnloadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<ReloadCommandMessage>::TYPE_ID => {
                    debug!("ReloadCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<ReloadCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetCursorCommandMessage>::TYPE_ID => {
                    debug!("SetCursorCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<SetCursorCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetErrorCommandMessage>::TYPE_ID => {
                    debug!("SetErrorCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<SetErrorCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SetPropCommandMessage>::TYPE_ID => {
                    debug!("SetPropCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<SetPropCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<SwitchXkbLayoutCommandMessage>::TYPE_ID => {
                    debug!("SwitchXkbLayoutCommandMessage");
                    MessageHandler::<FfiEnvelopePayload<SwitchXkbLayoutCommandMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<HyprlandStateRequestMessage>::TYPE_ID => {
                    debug!("HyprlandStateRequestMessage");
                    MessageHandler::<FfiEnvelopePayload<HyprlandStateRequestMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID => {
                    debug!("InvokeToolMessage");
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                }
                id if id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID => {
                    debug!("InvokeResourceMessage");
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
                _ => {
                    trace!("Hyprland service: unhandled message type for topic {}", envelope.topic.to_string());
                }
            }
        }
    }
}
