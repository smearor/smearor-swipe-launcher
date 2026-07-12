use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_CTL: &str = "service.hyprland.ctl";

/// Enters kill mode (similar to xkill).
#[derive(Clone, Debug, Default)]
pub struct KillCommandMessage;

/// ABI-stable version of `KillCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct KillCommandMessageStabby;

impl From<KillCommandMessage> for KillCommandMessageStabby {
    fn from(_value: KillCommandMessage) -> Self {
        Self
    }
}

impl From<KillCommandMessageStabby> for KillCommandMessage {
    fn from(_value: KillCommandMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for KillCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KillCommandMessage");
}

impl TypedMessage for KillCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KillCommandMessageStabby");
}

impl MessageTopic for KillCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for KillCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for KillCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
