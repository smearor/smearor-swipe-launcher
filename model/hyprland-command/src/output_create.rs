use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;
use smearor_hyprland_shared::HyprlandOutputBackend;

/// Creates a virtual output/display.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputCreateCommandMessage {
    /// The backend to use for the virtual output.
    pub backend: HyprlandOutputBackend,
}

impl TypedMessage for OutputCreateCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::OutputCreateCommandMessage");
}

impl MessageTopic for OutputCreateCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for OutputCreateCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
