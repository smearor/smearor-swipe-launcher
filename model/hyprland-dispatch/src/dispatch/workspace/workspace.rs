use serde::Deserialize;
use serde::Serialize;
use smearor_hyprland_shared::HyprlandWorkspaceIdentifierWithSpecial;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_DISPATCH: &str = "service.hyprland.dispatch";

/// Switches to the specified workspace.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceDispatchMessage {
    pub identifier: HyprlandWorkspaceIdentifierWithSpecial,
}

impl TypedMessage for WorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceDispatchMessage");
}

impl MessageTopic for WorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for WorkspaceDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
