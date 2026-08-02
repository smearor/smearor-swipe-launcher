use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandWorkspaceOptions;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Toggles workspace options (all pseudo or all float).
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceOptionDispatchMessage {
    pub option: HyprlandWorkspaceOptions,
}

impl TypedMessage for WorkspaceOptionDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::WorkspaceOptionDispatchMessage");
}

impl MessageTopic for WorkspaceOptionDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for WorkspaceOptionDispatchMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
