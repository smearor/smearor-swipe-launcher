use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for workspace lifecycle events broadcast by compositor services.
pub const TOPIC_WORKSPACE_LIFECYCLE: &str = "compositor.workspace.lifecycle";

/// Type of workspace lifecycle event.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceLifecycleType {
    /// Workspace was created.
    #[default]
    Created,
    /// Workspace was destroyed.
    Destroyed,
}

/// Event broadcast when a workspace is created or destroyed.
///
/// Useful for widgets that display workspace lists or for the launcher to track
/// available workspaces.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceLifecycleEvent {
    /// The workspace name or number.
    pub workspace_name: stabby::string::String,
    /// The workspace ID (numeric, as reported by the compositor).
    pub workspace_id: i32,
    /// The monitor index the workspace is on, if known.
    pub monitor_index: u32,
    /// Whether the workspace was created or destroyed.
    pub lifecycle_type: WorkspaceLifecycleType,
}

impl TypedMessage for WorkspaceLifecycleEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::WorkspaceLifecycleEvent");
}

impl MessageTopic for WorkspaceLifecycleEvent {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_LIFECYCLE
    }
}

impl SharedMessage for WorkspaceLifecycleEvent {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_LIFECYCLE
    }
}
