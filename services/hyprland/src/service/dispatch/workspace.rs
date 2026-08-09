use crate::service::converters::monitor_identifier::convert_monitor_identifier;
use crate::service::converters::window_identifier::convert_window_identifier_opt;
use crate::service::converters::workspace_identifier::OwnedWorkspaceIdentifierWithSpecial;
use crate::service::converters::workspace_options::convert_workspace_options;
use crate::service::ensure_hyprland_instance_signature;
use hyprland::dispatch::Dispatch;
use hyprland::dispatch::DispatchType;
use hyprland::dispatch::FirstEmpty;
use hyprland::dispatch::WorkspaceIdentifierWithSpecial;
use smearor_hyprland_model::HyprlandWorkspaceDispatchMessage;
use smearor_hyprland_model::HyprlandWorkspaceIdentifierKind;
use smearor_hyprland_model::MoveCurrentWorkspaceToMonitorDispatchMessage;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceDispatchMessage;
use smearor_hyprland_model::MoveFocusedWindowToWorkspaceSilentDispatchMessage;
use smearor_hyprland_model::MoveToWorkspaceDispatchMessage;
use smearor_hyprland_model::MoveToWorkspaceSilentDispatchMessage;
use smearor_hyprland_model::RenameWorkspaceDispatchMessage;
use smearor_hyprland_model::SwapActiveWorkspacesDispatchMessage;
use smearor_hyprland_model::ToggleSpecialWorkspaceDispatchMessage;
use smearor_hyprland_model::WorkspaceDispatchKind;
use smearor_hyprland_model::WorkspaceDispatchMessage;
use smearor_hyprland_model::WorkspaceDispatchOps;
use smearor_hyprland_model::WorkspaceOptionDispatchMessage;
use smearor_model_compositor::CreateWorkspaceMessage;
use smearor_model_compositor::SwitchWorkspaceMessage;
use smearor_model_compositor::WorkspaceCreatePosition;
use tracing::error;
use tracing::trace;

pub(crate) async fn handle_dispatch_workspace(message: HyprlandWorkspaceDispatchMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = handle_workspace_dispatch(message.kind, message.ops).await {
        error!("Hyprland workspace dispatch failed: {error}");
    }
}

async fn handle_workspace_dispatch(kind: WorkspaceDispatchKind, ops: WorkspaceDispatchOps) -> hyprland::Result<()> {
    match kind {
        WorkspaceDispatchKind::MoveCurrentWorkspaceToMonitor => match ops.move_current_workspace_to_monitor.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_current_workspace_to_monitor(payload).await,
            None => Ok(()),
        },
        WorkspaceDispatchKind::MoveFocusedWindowToWorkspace => match ops.move_focused_window_to_workspace.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_focused_window_to_workspace(payload).await,
            None => Ok(()),
        },
        WorkspaceDispatchKind::MoveFocusedWindowToWorkspaceSilent => {
            match ops.move_focused_window_to_workspace_silent.match_owned(|value| Some(value.into()), || None) {
                Some(payload) => handle_move_focused_window_to_workspace_silent(payload).await,
                None => Ok(()),
            }
        }
        WorkspaceDispatchKind::MoveToWorkspace => match ops.move_to_workspace.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_to_workspace(payload).await,
            None => Ok(()),
        },
        WorkspaceDispatchKind::MoveToWorkspaceSilent => match ops.move_to_workspace_silent.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_move_to_workspace_silent(payload).await,
            None => Ok(()),
        },
        WorkspaceDispatchKind::RenameWorkspace => match ops.rename_workspace.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_rename_workspace(payload).await,
            None => Ok(()),
        },
        WorkspaceDispatchKind::SwapActiveWorkspaces => match ops.swap_active_workspaces.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_swap_active_workspaces(payload).await,
            None => Ok(()),
        },
        WorkspaceDispatchKind::ToggleSpecialWorkspace => match ops.toggle_special_workspace.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_toggle_special_workspace(payload).await,
            None => Ok(()),
        },
        WorkspaceDispatchKind::Workspace => match ops.workspace.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_workspace(payload).await,
            None => Ok(()),
        },
        WorkspaceDispatchKind::WorkspaceOption => match ops.workspace_option.match_owned(|value| Some(value.into()), || None) {
            Some(payload) => handle_workspace_option(payload).await,
            None => Ok(()),
        },
    }
}

async fn handle_move_to_workspace(payload: MoveToWorkspaceDispatchMessage) -> hyprland::Result<()> {
    trace!(
        "Hyprland service: dispatching move to workspace: kind={:?}, id={}",
        payload.identifier.kind, payload.identifier.id
    );
    match payload.identifier.kind {
        HyprlandWorkspaceIdentifierKind::Id => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(WorkspaceIdentifierWithSpecial::Id(payload.identifier.id), None)).await
        }
        HyprlandWorkspaceIdentifierKind::Relative => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(WorkspaceIdentifierWithSpecial::Relative(payload.identifier.id), None)).await
        }
        HyprlandWorkspaceIdentifierKind::RelativeMonitor => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(WorkspaceIdentifierWithSpecial::RelativeMonitor(payload.identifier.id), None)).await
        }
        HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(
                WorkspaceIdentifierWithSpecial::RelativeMonitorIncludingEmpty(payload.identifier.id),
                None,
            ))
            .await
        }
        HyprlandWorkspaceIdentifierKind::RelativeOpen => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(WorkspaceIdentifierWithSpecial::RelativeOpen(payload.identifier.id), None)).await
        }
        HyprlandWorkspaceIdentifierKind::Previous => Dispatch::call_async(DispatchType::MoveToWorkspace(WorkspaceIdentifierWithSpecial::Previous, None)).await,
        HyprlandWorkspaceIdentifierKind::Empty => {
            Dispatch::call_async(DispatchType::MoveToWorkspace(
                WorkspaceIdentifierWithSpecial::Empty(FirstEmpty {
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
            Dispatch::call_async(DispatchType::MoveToWorkspace(WorkspaceIdentifierWithSpecial::Name(name_ref), None)).await
        }
        HyprlandWorkspaceIdentifierKind::Special => {
            let opt: Option<stabby::string::String> = payload.identifier.special_name.clone().into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str());
            Dispatch::call_async(DispatchType::MoveToWorkspace(WorkspaceIdentifierWithSpecial::Special(name_ref), None)).await
        }
    }
}

async fn handle_workspace(payload: WorkspaceDispatchMessage) -> hyprland::Result<()> {
    trace!(
        "Hyprland service: dispatching workspace change: kind={:?}, id={}",
        payload.identifier.kind, payload.identifier.id
    );
    match payload.identifier.kind {
        HyprlandWorkspaceIdentifierKind::Id => Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(payload.identifier.id))).await,
        HyprlandWorkspaceIdentifierKind::Relative => {
            Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Relative(payload.identifier.id))).await
        }
        HyprlandWorkspaceIdentifierKind::RelativeMonitor => {
            Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::RelativeMonitor(payload.identifier.id))).await
        }
        HyprlandWorkspaceIdentifierKind::RelativeMonitorIncludingEmpty => {
            Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::RelativeMonitorIncludingEmpty(payload.identifier.id))).await
        }
        HyprlandWorkspaceIdentifierKind::RelativeOpen => {
            Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::RelativeOpen(payload.identifier.id))).await
        }
        HyprlandWorkspaceIdentifierKind::Previous => Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Previous)).await,
        HyprlandWorkspaceIdentifierKind::Empty => {
            Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Empty(FirstEmpty {
                on_monitor: false,
                next: false,
            })))
            .await
        }
        HyprlandWorkspaceIdentifierKind::Name => {
            let opt: Option<stabby::string::String> = payload.identifier.name.clone().into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str()).unwrap_or("");
            Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Name(name_ref))).await
        }
        HyprlandWorkspaceIdentifierKind::Special => {
            let opt: Option<stabby::string::String> = payload.identifier.special_name.clone().into();
            let name_string = opt.map(|name| name.to_string());
            let name_ref = name_string.as_ref().map(|name| name.as_str());
            Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Special(name_ref))).await
        }
    }
}

async fn handle_move_current_workspace_to_monitor(payload: MoveCurrentWorkspaceToMonitorDispatchMessage) -> hyprland::Result<()> {
    let name_opt: Option<stabby::string::String> = payload.monitor_identifier.name.clone().into();
    let name_string = name_opt.map(|n| n.to_string());
    let name_ref = name_string.as_ref().map(|n| n.as_str());
    let monitor = convert_monitor_identifier(&payload.monitor_identifier, name_ref);
    Dispatch::call_async(DispatchType::MoveCurrentWorkspaceToMonitor(monitor)).await
}

async fn handle_move_focused_window_to_workspace(payload: MoveFocusedWindowToWorkspaceDispatchMessage) -> hyprland::Result<()> {
    let ws_id = OwnedWorkspaceIdentifierWithSpecial::from(&payload.identifier);
    Dispatch::call_async(DispatchType::MoveToWorkspace(ws_id.as_ref(), None)).await
}

async fn handle_move_focused_window_to_workspace_silent(payload: MoveFocusedWindowToWorkspaceSilentDispatchMessage) -> hyprland::Result<()> {
    let ws_id = OwnedWorkspaceIdentifierWithSpecial::from(&payload.identifier);
    Dispatch::call_async(DispatchType::MoveToWorkspaceSilent(ws_id.as_ref(), None)).await
}

async fn handle_move_to_workspace_silent(payload: MoveToWorkspaceSilentDispatchMessage) -> hyprland::Result<()> {
    let ws_id = OwnedWorkspaceIdentifierWithSpecial::from(&payload.identifier);
    let win_id = convert_window_identifier_opt(&payload.window_identifier);
    let win_id = win_id.as_ref().map(|w| w.as_ref());
    Dispatch::call_async(DispatchType::MoveToWorkspaceSilent(ws_id.as_ref(), win_id)).await
}

async fn handle_rename_workspace(payload: RenameWorkspaceDispatchMessage) -> hyprland::Result<()> {
    let name_ref = payload.new_name.as_ref().map(|n| n.as_str());
    Dispatch::call_async(DispatchType::RenameWorkspace(payload.workspace_id, name_ref)).await
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

async fn handle_toggle_special_workspace(payload: ToggleSpecialWorkspaceDispatchMessage) -> hyprland::Result<()> {
    Dispatch::call_async(DispatchType::ToggleSpecialWorkspace(payload.workspace_name)).await
}

async fn handle_workspace_option(payload: WorkspaceOptionDispatchMessage) -> hyprland::Result<()> {
    let opt = convert_workspace_options(payload.option);
    Dispatch::call_async(DispatchType::WorkspaceOption(opt)).await
}

/// Handle a compositor-unified SwitchWorkspaceMessage by translating it to a
/// Hyprland workspace dispatch.
pub(crate) async fn handle_switch_workspace(message: SwitchWorkspaceMessage) {
    ensure_hyprland_instance_signature();
    trace!("Hyprland service: switching to workspace {}", message.workspace_id);
    if let Err(error) = Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(message.workspace_id))).await {
        error!("Hyprland service: failed to switch workspace: {error}");
    }
}

/// Handle a compositor-unified CreateWorkspaceMessage.
///
/// Hyprland supports creating workspaces by dispatching to a new workspace ID.
/// For `After`, we use `workspace +1` relative to the reference workspace.
/// For `Before`, we use `workspace -1`.
pub(crate) async fn handle_create_workspace(message: CreateWorkspaceMessage) {
    ensure_hyprland_instance_signature();
    let new_id = match message.position {
        WorkspaceCreatePosition::After => message.relative_to + 1,
        WorkspaceCreatePosition::Before => message.relative_to - 1,
    };
    trace!(
        "Hyprland service: creating workspace at {} (relative_to={}, position={:?})",
        new_id, message.relative_to, message.position
    );
    if let Err(error) = Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(new_id))).await {
        error!("Hyprland service: failed to create workspace: {error}");
    }
}
