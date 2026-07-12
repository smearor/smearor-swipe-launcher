use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::workspace::TOPIC_DISPATCH;

/// Toggles DPMS (display power management) state.
#[derive(Clone, Debug, Default)]
pub struct ToggleDpmsDispatchMessage {
    pub on: bool,
    pub name: Option<String>,
}

/// ABI-stable version of `ToggleDpmsDispatchMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ToggleDpmsDispatchMessageStabby {
    pub on: bool,
    pub name: stabby::option::Option<stabby::string::String>,
}

impl From<ToggleDpmsDispatchMessage> for ToggleDpmsDispatchMessageStabby {
    fn from(value: ToggleDpmsDispatchMessage) -> Self {
        Self {
            on: value.on,
            name: value.name.map(stabby::string::String::from).into(),
        }
    }
}

impl From<ToggleDpmsDispatchMessageStabby> for ToggleDpmsDispatchMessage {
    fn from(value: ToggleDpmsDispatchMessageStabby) -> Self {
        let name: Option<stabby::string::String> = value.name.into();
        Self {
            on: value.on,
            name: name.map(|s| s.to_string()),
        }
    }
}

impl TypedMessage for ToggleDpmsDispatchMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleDpmsDispatchMessage");
}

impl TypedMessage for ToggleDpmsDispatchMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ToggleDpmsDispatchMessageStabby");
}

impl MessageTopic for ToggleDpmsDispatchMessage {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl MessageTopic for ToggleDpmsDispatchMessageStabby {
    fn topic() -> &'static str {
        TOPIC_DISPATCH
    }
}

impl SharedMessage for ToggleDpmsDispatchMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_DISPATCH
    }
}
