use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for workspace snapshot requests (Widget -> Service).
pub const TOPIC_WORKSPACE_SNAPSHOT_REQUEST: &str = "compositor.workspace.snapshot.request";

/// Request a workspace snapshot from the active compositor service.
///
/// Sent by the widget on startup to request the current workspace list.
#[stabby::stabby]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceSnapshotRequestMessage {
    /// The monitor index the widget is interested in.
    /// Set to `0` if the widget does not filter by monitor.
    pub monitor_index: u32,
}

impl TypedMessage for WorkspaceSnapshotRequestMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::WorkspaceSnapshotRequestMessage");
}

impl MessageTopic for WorkspaceSnapshotRequestMessage {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_SNAPSHOT_REQUEST
    }
}

impl SharedMessage for WorkspaceSnapshotRequestMessage {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_SNAPSHOT_REQUEST
    }
}
