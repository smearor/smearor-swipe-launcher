use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;
use crate::HyprlandOutputBackend;

/// Creates a virtual output/display.
#[derive(Clone, Debug, Default)]
pub struct OutputCreateCommandMessage {
    /// The backend to use for the virtual output.
    pub backend: HyprlandOutputBackend,
}

/// ABI-stable version of `OutputCreateCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct OutputCreateCommandMessageStabby {
    /// The backend to use for the virtual output.
    pub backend: HyprlandOutputBackend,
}

impl From<OutputCreateCommandMessage> for OutputCreateCommandMessageStabby {
    fn from(value: OutputCreateCommandMessage) -> Self {
        Self { backend: value.backend }
    }
}

impl From<OutputCreateCommandMessageStabby> for OutputCreateCommandMessage {
    fn from(value: OutputCreateCommandMessageStabby) -> Self {
        Self { backend: value.backend }
    }
}

impl TypedMessage for OutputCreateCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OutputCreateCommandMessage");
}

impl TypedMessage for OutputCreateCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OutputCreateCommandMessageStabby");
}

impl MessageTopic for OutputCreateCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for OutputCreateCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for OutputCreateCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
