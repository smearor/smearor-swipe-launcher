use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use smearor_hyprland_shared::HyprlandWindowIdentifier;
use smearor_hyprland_shared::HyprlandWorkspaceIdentifierWithSpecial;

use crate::dispatch::workspace::TOPIC_DISPATCH;

/// Moves a window to a workspace silently (without switching to it).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveToWorkspaceSilentDispatchMessage {
    pub identifier: HyprlandWorkspaceIdentifierWithSpecial,
    pub window_identifier: Option<HyprlandWindowIdentifier>,
}

/// ABI-stable version of `MoveToWorkspaceSilentDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MoveToWorkspaceSilentDispatchMessageStabby {
    pub identifier: HyprlandWorkspaceIdentifierWithSpecial,
    pub window_identifier: stabby::option::Option<HyprlandWindowIdentifier>,
}

impl From<MoveToWorkspaceSilentDispatchMessage> for MoveToWorkspaceSilentDispatchMessageStabby {
    fn from(value: MoveToWorkspaceSilentDispatchMessage) -> Self {
        Self {
            identifier: value.identifier,
            window_identifier: value.window_identifier.into(),
        }
    }
}

impl From<MoveToWorkspaceSilentDispatchMessageStabby> for MoveToWorkspaceSilentDispatchMessage {
    fn from(value: MoveToWorkspaceSilentDispatchMessageStabby) -> Self {
        Self {
            identifier: value.identifier,
            window_identifier: value.window_identifier.into(),
        }
    }
}

impl TypedMessage for MoveToWorkspaceSilentDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveToWorkspaceSilentDispatchMessage");
}

impl TypedMessage for MoveToWorkspaceSilentDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::MoveToWorkspaceSilentDispatchMessageStabby");
}

impl MessageTopic for MoveToWorkspaceSilentDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for MoveToWorkspaceSilentDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for MoveToWorkspaceSilentDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
