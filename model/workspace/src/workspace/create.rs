use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::workspace::create_position::WorkspaceCreatePosition;

/// Topic for workspace creation commands (Widget -> Service).
pub const TOPIC_CREATE_WORKSPACE: &str = "compositor.workspace.create";

/// Command to create a new workspace.
///
/// Compositor-unified message sent by the widget when the user swipes past
/// the first or last workspace. The service creates a new workspace relative
/// to the reference workspace.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
