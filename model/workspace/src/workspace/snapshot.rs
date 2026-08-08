use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::workspace::workspace_info::WorkspaceInfo;

/// Topic for workspace snapshot responses (Service -> Widget).
pub const TOPIC_WORKSPACE_SNAPSHOT: &str = "compositor.workspace.snapshot";

/// Snapshot of all workspaces, sent from the service to the widget.
///
/// Broadcast by the service in response to a snapshot request,
/// or automatically on startup.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceSnapshotMessage {
    /// All known workspaces, ordered by workspace ID.
    pub workspaces: stabby::vec::Vec<WorkspaceInfo>,
    /// The currently active workspace ID.
    pub active_workspace_id: i32,
    /// The monitor index on which the active workspace is located.
    pub active_monitor_index: u32,
}

impl TypedMessage for WorkspaceSnapshotMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::WorkspaceSnapshotMessage");
}

impl MessageTopic for WorkspaceSnapshotMessage {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_SNAPSHOT
    }
}

impl SharedMessage for WorkspaceSnapshotMessage {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_SNAPSHOT
    }
}
