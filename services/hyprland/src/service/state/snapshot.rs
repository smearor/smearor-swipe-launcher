use crate::service::ensure_hyprland_instance_signature;
use crate::service::shared_state::HyprlandSharedState;
use hyprland::shared::HyprData;
use hyprland::shared::HyprDataActive;
use smearor_model_compositor::WorkspaceInfo;
use smearor_model_compositor::WorkspaceSnapshotMessage;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::debug;
use tracing::error;

/// Handle a WorkspaceSnapshotRequestMessage by querying Hyprland for all
/// workspaces and broadcasting a WorkspaceSnapshotMessage.
pub(crate) async fn handle_snapshot_request(
    _message: WorkspaceSnapshotRequestMessage,
    broadcaster: &MessageBroadcasterInner,
    shared_state: &Arc<Mutex<HyprlandSharedState>>,
) {
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

    if let Ok(mut guard) = shared_state.lock() {
        guard.workspace_snapshot = Some(snapshot.clone());
    }

    broadcaster.broadcast_message_to_topic(snapshot);
}
