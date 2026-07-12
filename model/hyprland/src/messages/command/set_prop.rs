use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;
use crate::HyprlandPropType;

/// Sets a window property.
#[derive(Clone, Debug, Default)]
pub struct SetPropCommandMessage {
    /// The window identifier (e.g. "address:0x1234" or "title:My Window").
    pub identifier: String,
    /// The property to set.
    pub prop: HyprlandPropType,
    /// Whether to lock the property.
    pub lock: bool,
}

/// ABI-stable version of `SetPropCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct SetPropCommandMessageStabby {
    /// The window identifier (e.g. "address:0x1234" or "title:My Window").
    pub identifier: stabby::string::String,
    /// The property to set.
    pub prop: HyprlandPropType,
    /// Whether to lock the property.
    pub lock: bool,
}

impl From<SetPropCommandMessage> for SetPropCommandMessageStabby {
    fn from(value: SetPropCommandMessage) -> Self {
        Self {
            identifier: value.identifier.into(),
            prop: value.prop,
            lock: value.lock,
        }
    }
}

impl From<SetPropCommandMessageStabby> for SetPropCommandMessage {
    fn from(value: SetPropCommandMessageStabby) -> Self {
        Self {
            identifier: value.identifier.to_string(),
            prop: value.prop,
            lock: value.lock,
        }
    }
}

impl TypedMessage for SetPropCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SetPropCommandMessage");
}

impl TypedMessage for SetPropCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::SetPropCommandMessageStabby");
}

impl MessageTopic for SetPropCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for SetPropCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for SetPropCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
