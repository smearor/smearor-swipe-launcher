use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Reloads the Hyprland configuration.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReloadCommandMessage;

impl TypedMessage for ReloadCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::ReloadCommandMessage");
}

impl MessageTopic for ReloadCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for ReloadCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
