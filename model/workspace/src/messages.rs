use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for workspace change events broadcast by compositor services.
pub const TOPIC_WORKSPACE_CHANGED: &str = "compositor::workspace_changed";

/// Event broadcast when the active workspace changes on a monitor.
///
/// Launcher instances use this to re-evaluate layout profiles with
/// `LayoutTrigger::Workspace` or `MonitorIndexWorkspace`.
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub struct WorkspaceChangedEvent {
    /// The workspace name or number that became active.
    pub workspace_name: stabby::string::String,
    /// The workspace ID (numeric, as reported by the compositor).
    /// Set to `-1` for special workspaces.
    pub workspace_id: i32,
    /// The monitor index (0-based, matching GDK display order) on which the
    /// workspace change occurred.
    pub monitor_index: u32,
}

impl TypedMessage for WorkspaceChangedEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::WorkspaceChangedEvent");
}

impl MessageTopic for WorkspaceChangedEvent {
    fn topic() -> &'static str {
        TOPIC_WORKSPACE_CHANGED
    }
}

impl SharedMessage for WorkspaceChangedEvent {
    fn topic(&self) -> &'static str {
        TOPIC_WORKSPACE_CHANGED
    }
}
