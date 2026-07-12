use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;
use crate::HyprlandSwitchXkbLayoutCmd;

/// Switches the XKB keyboard layout for a device.
#[derive(Clone, Debug, Default)]
pub struct SwitchXkbLayoutCommandMessage {
    /// The keyboard device name.
    pub device: String,
    /// The layout switch command.
    pub cmd: HyprlandSwitchXkbLayoutCmd,
}

/// ABI-stable version of `SwitchXkbLayoutCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SwitchXkbLayoutCommandMessageStabby {
    /// The keyboard device name.
    pub device: stabby::string::String,
    /// The layout switch command.
    pub cmd: HyprlandSwitchXkbLayoutCmd,
}

impl From<SwitchXkbLayoutCommandMessage> for SwitchXkbLayoutCommandMessageStabby {
    fn from(value: SwitchXkbLayoutCommandMessage) -> Self {
        Self {
            device: value.device.into(),
            cmd: value.cmd,
        }
    }
}

impl From<SwitchXkbLayoutCommandMessageStabby> for SwitchXkbLayoutCommandMessage {
    fn from(value: SwitchXkbLayoutCommandMessageStabby) -> Self {
        Self {
            device: value.device.to_string(),
            cmd: value.cmd,
        }
    }
}

impl TypedMessage for SwitchXkbLayoutCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwitchXkbLayoutCommandMessage");
}

impl TypedMessage for SwitchXkbLayoutCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SwitchXkbLayoutCommandMessageStabby");
}

impl MessageTopic for SwitchXkbLayoutCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for SwitchXkbLayoutCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for SwitchXkbLayoutCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
