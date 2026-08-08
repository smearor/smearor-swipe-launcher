use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

/// Topic for workspace switch commands (Widget -> Service).
pub const TOPIC_SWITCH_WORKSPACE: &str = "compositor.workspace.switch";

/// Command to switch to a specific workspace.
///
/// Compositor-unified message sent by the widget to request a workspace change.
/// The active compositor service translates this to the compositor-specific
/// dispatch mechanism.
#[stabby::stabby]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
