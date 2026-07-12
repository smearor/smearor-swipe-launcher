use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Removes a virtual output/display.
#[derive(Clone, Debug, Default)]
pub struct OutputRemoveCommandMessage {
    /// The name of the virtual output to remove.
    pub name: String,
}

/// ABI-stable version of `OutputRemoveCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OutputRemoveCommandMessageStabby {
    /// The name of the virtual output to remove.
    pub name: stabby::string::String,
}

impl From<OutputRemoveCommandMessage> for OutputRemoveCommandMessageStabby {
    fn from(value: OutputRemoveCommandMessage) -> Self {
        Self { name: value.name.into() }
    }
}

impl From<OutputRemoveCommandMessageStabby> for OutputRemoveCommandMessage {
    fn from(value: OutputRemoveCommandMessageStabby) -> Self {
        Self { name: value.name.to_string() }
    }
}

impl TypedMessage for OutputRemoveCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OutputRemoveCommandMessage");
}

impl TypedMessage for OutputRemoveCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OutputRemoveCommandMessageStabby");
}

impl MessageTopic for OutputRemoveCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for OutputRemoveCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for OutputRemoveCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
