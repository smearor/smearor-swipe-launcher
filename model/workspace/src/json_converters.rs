use smearor_swipe_launcher_plugin_api::FfiCoreContext;

use crate::CreateWorkspaceMessage;
use crate::SwitchWorkspaceMessage;
use crate::WorkspaceCreatePosition;
use crate::WorkspaceInfo;
use crate::WorkspaceSnapshotMessage;
use crate::WorkspaceSnapshotRequestMessage;

fn parse_workspace_create_position(value: &serde_json::Value) -> WorkspaceCreatePosition {
    match value.as_str() {
        Some("After") => WorkspaceCreatePosition::After,
        _ => WorkspaceCreatePosition::Before,
    }
}

smearor_swipe_launcher_plugin_api::impl_json_convertible!(SwitchWorkspaceMessageConverter, SwitchWorkspaceMessage, |json: serde_json::Value| {
    let workspace_id = json.get("workspace_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    SwitchWorkspaceMessage { workspace_id }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(CreateWorkspaceMessageConverter, CreateWorkspaceMessage, |json: serde_json::Value| {
    let relative_to = json.get("relative_to").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let position = parse_workspace_create_position(json.get("position").unwrap_or(&serde_json::Value::Null));
    CreateWorkspaceMessage { relative_to, position }
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(
    WorkspaceSnapshotRequestMessageConverter,
    WorkspaceSnapshotRequestMessage,
    |json: serde_json::Value| {
        let monitor_index = json.get("monitor_index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        WorkspaceSnapshotRequestMessage { monitor_index }
    }
);

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WorkspaceSnapshotMessageConverter, WorkspaceSnapshotMessage, |json: serde_json::Value| {
    let active_workspace_id = json.get("active_workspace_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let active_monitor_index = json.get("active_monitor_index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    let mut workspaces: Vec<WorkspaceInfo> = Vec::new();
    if let Some(arr) = json.get("workspaces").and_then(|v| v.as_array()) {
        for ws in arr {
            let workspace_id = ws.get("workspace_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let workspace_name = ws.get("workspace_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let monitor_index = ws.get("monitor_index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let is_active = ws.get("is_active").and_then(|v| v.as_bool()).unwrap_or(false);
            workspaces.push(WorkspaceInfo {
                workspace_id,
                workspace_name: workspace_name.into(),
                monitor_index,
                is_active,
            });
        }
    }

    WorkspaceSnapshotMessage {
        workspaces: workspaces.into_iter().collect(),
        active_workspace_id,
        active_monitor_index,
    }
});

/// Register all JSON converter implementations for compositor switcher messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    SwitchWorkspaceMessageConverter::register_in_host(context);
    CreateWorkspaceMessageConverter::register_in_host(context);
    WorkspaceSnapshotRequestMessageConverter::register_in_host(context);
    WorkspaceSnapshotMessageConverter::register_in_host(context);
}
