use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Renames a workspace by ID with an optional new name.
#[derive(Clone, Debug, Default)]
pub struct RenameWorkspaceDispatchMessage {
    pub workspace_id: i32,
    pub new_name: Option<String>,
}

/// ABI-stable version of `RenameWorkspaceDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct RenameWorkspaceDispatchMessageStabby {
    pub workspace_id: i32,
    pub new_name: stabby::option::Option<stabby::string::String>,
}

impl From<RenameWorkspaceDispatchMessage> for RenameWorkspaceDispatchMessageStabby {
    fn from(value: RenameWorkspaceDispatchMessage) -> Self {
        Self {
            workspace_id: value.workspace_id,
            new_name: value.new_name.map(stabby::string::String::from).into(),
        }
    }
}

impl From<RenameWorkspaceDispatchMessageStabby> for RenameWorkspaceDispatchMessage {
    fn from(value: RenameWorkspaceDispatchMessageStabby) -> Self {
        let new_name: Option<stabby::string::String> = value.new_name.into();
        Self {
            workspace_id: value.workspace_id,
            new_name: new_name.map(|s| s.to_string()),
        }
    }
}

impl TypedMessage for RenameWorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::RenameWorkspaceDispatchMessage");
}

impl TypedMessage for RenameWorkspaceDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::RenameWorkspaceDispatchMessageStabby");
}

impl MessageTopic for RenameWorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for RenameWorkspaceDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for RenameWorkspaceDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
