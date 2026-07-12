use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles the special workspace with an optional name.
#[derive(Clone, Debug, Default)]
pub struct ToggleSpecialWorkspaceDispatchMessage {
    pub workspace_name: Option<String>,
}

/// ABI-stable version of `ToggleSpecialWorkspaceDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ToggleSpecialWorkspaceDispatchMessageStabby {
    pub workspace_name: stabby::option::Option<stabby::string::String>,
}

impl From<ToggleSpecialWorkspaceDispatchMessage> for ToggleSpecialWorkspaceDispatchMessageStabby {
    fn from(value: ToggleSpecialWorkspaceDispatchMessage) -> Self {
        Self {
            workspace_name: value.workspace_name.map(stabby::string::String::from).into(),
        }
    }
}

impl From<ToggleSpecialWorkspaceDispatchMessageStabby> for ToggleSpecialWorkspaceDispatchMessage {
    fn from(value: ToggleSpecialWorkspaceDispatchMessageStabby) -> Self {
        let workspace_name: Option<stabby::string::String> = value.workspace_name.into();
        Self {
            workspace_name: workspace_name.map(|s| s.to_string()),
        }
    }
}

impl TypedMessage for ToggleSpecialWorkspaceDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleSpecialWorkspaceDispatchMessage");
}

impl TypedMessage for ToggleSpecialWorkspaceDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleSpecialWorkspaceDispatchMessageStabby");
}

impl MessageTopic for ToggleSpecialWorkspaceDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ToggleSpecialWorkspaceDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleSpecialWorkspaceDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
