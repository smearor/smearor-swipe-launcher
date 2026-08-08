mod monitor;
mod workspace;

use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::impl_json_convertible;

pub use monitor::MonitorChangeType;
pub use monitor::MonitorChangedEvent;
pub use monitor::TOPIC_MONITOR_CHANGED;
pub use workspace::CreateWorkspaceMessage;
pub use workspace::SwitchWorkspaceMessage;
pub use workspace::TOPIC_CREATE_WORKSPACE;
pub use workspace::TOPIC_SWITCH_WORKSPACE;
pub use workspace::TOPIC_WORKSPACE_CHANGED;
pub use workspace::TOPIC_WORKSPACE_LIFECYCLE;
pub use workspace::TOPIC_WORKSPACE_SNAPSHOT;
pub use workspace::TOPIC_WORKSPACE_SNAPSHOT_REQUEST;
pub use workspace::WorkspaceChangedEvent;
pub use workspace::WorkspaceCreatePosition;
pub use workspace::WorkspaceInfo;
pub use workspace::WorkspaceLifecycleEvent;
pub use workspace::WorkspaceLifecycleType;
pub use workspace::WorkspaceSnapshotMessage;
pub use workspace::WorkspaceSnapshotRequestMessage;

impl_json_convertible!(SwitchWorkspaceMessageConverter, SwitchWorkspaceMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());
impl_json_convertible!(CreateWorkspaceMessageConverter, CreateWorkspaceMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());
impl_json_convertible!(WorkspaceSnapshotRequestMessageConverter, WorkspaceSnapshotRequestMessage, |json: serde_json::Value| {
    serde_json::from_value(json).unwrap_or_default()
});
impl_json_convertible!(WorkspaceSnapshotMessageConverter, WorkspaceSnapshotMessage, |json: serde_json::Value| serde_json::from_value(
    json
)
.unwrap_or_default());

/// Register all JSON converter implementations for compositor switcher messages.
///
/// Call this once during plugin initialisation.
pub fn register_json_converters(context: Option<FfiCoreContext>) {
    SwitchWorkspaceMessageConverter::register_in_host(context);
    CreateWorkspaceMessageConverter::register_in_host(context);
    WorkspaceSnapshotRequestMessageConverter::register_in_host(context);
    WorkspaceSnapshotMessageConverter::register_in_host(context);
}
