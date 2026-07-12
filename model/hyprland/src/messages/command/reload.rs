use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Reloads the Hyprland configuration.
#[derive(Clone, Debug, Default)]
pub struct ReloadCommandMessage;

/// ABI-stable version of `ReloadCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct ReloadCommandMessageStabby;

impl From<ReloadCommandMessage> for ReloadCommandMessageStabby {
    fn from(_value: ReloadCommandMessage) -> Self {
        Self
    }
}

impl From<ReloadCommandMessageStabby> for ReloadCommandMessage {
    fn from(_value: ReloadCommandMessageStabby) -> Self {
        Self
    }
}

impl TypedMessage for ReloadCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ReloadCommandMessage");
}

impl TypedMessage for ReloadCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ReloadCommandMessageStabby");
}

impl MessageTopic for ReloadCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for ReloadCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for ReloadCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
