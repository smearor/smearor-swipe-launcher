use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_CTL: &str = "service.hyprland.ctl";

/// Enters kill mode (similar to xkill).
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KillCommandMessage;

impl TypedMessage for KillCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::KillCommandMessage");
}

impl MessageTopic for KillCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for KillCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
