use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for workspace snapshot requests (Widget -> Service).
pub const TOPIC_WORKSPACE_SNAPSHOT_REQUEST: &str = "compositor::workspace_snapshot_request";

/// Topic for workspace snapshot responses (Service -> Widget).
pub const TOPIC_WORKSPACE_SNAPSHOT: &str = "compositor::workspace_snapshot";

/// Topic for workspace switch commands (Widget -> Service).
pub const TOPIC_SWITCH_WORKSPACE: &str = "compositor::switch_workspace";

/// Topic for workspace creation commands (Widget -> Service).
pub const TOPIC_CREATE_WORKSPACE: &str = "compositor::create_workspace";

/// Information about a single workspace.
///
/// Used in snapshots and internal widget state to represent one workspace
/// in the compositor's workspace list.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceInfo {
    /// The workspace ID (numeric, as reported by the compositor).
    /// Set to `-1` for special workspaces.
    pub workspace_id: i32,
    /// The workspace name or number as a string.
    pub workspace_name: stabby::string::String,
    /// The monitor index (0-based, matching GDK display order) on which the
    /// workspace is located.
    pub monitor_index: u32,
    /// Whether this workspace is currently active.
    pub is_active: bool,
}

/// Snapshot of all workspaces, sent from the service to the widget.
///
/// Broadcast by the service in response to a snapshot request,
/// or automatically on startup.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
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

/// Request a workspace snapshot from the active compositor service.
///
/// Sent by the widget on startup to request the current workspace list.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
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

/// Command to switch to a specific workspace.
///
/// Compositor-unified message sent by the widget to request a workspace change.
/// The active compositor service translates this to the compositor-specific
/// dispatch mechanism.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct SwitchWorkspaceMessage {
    /// The workspace ID to switch to.
    pub workspace_id: i32,
}

impl TypedMessage for SwitchWorkspaceMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::SwitchWorkspaceMessage");
}

impl MessageTopic for SwitchWorkspaceMessage {
    fn topic() -> &'static str {
        TOPIC_SWITCH_WORKSPACE
    }
}

impl SharedMessage for SwitchWorkspaceMessage {
    fn topic(&self) -> &'static str {
        TOPIC_SWITCH_WORKSPACE
    }
}

/// Position for creating a new workspace relative to a reference workspace.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkspaceCreatePosition {
    /// Create the new workspace before the reference workspace.
    #[default]
    Before,
    /// Create the new workspace after the reference workspace.
    After,
}

/// Command to create a new workspace.
///
/// Compositor-unified message sent by the widget when the user swipes past
/// the first or last workspace. The service creates a new workspace relative
/// to the reference workspace.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct CreateWorkspaceMessage {
    /// The workspace ID of the reference workspace.
    pub relative_to: i32,
    /// Whether to create the new workspace before or after the reference.
    pub position: WorkspaceCreatePosition,
}

impl TypedMessage for CreateWorkspaceMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::CreateWorkspaceMessage");
}

impl MessageTopic for CreateWorkspaceMessage {
    fn topic() -> &'static str {
        TOPIC_CREATE_WORKSPACE
    }
}

impl SharedMessage for CreateWorkspaceMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CREATE_WORKSPACE
    }
}
