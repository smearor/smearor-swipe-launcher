use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Loads a Hyprland plugin by path.
#[derive(Clone, Debug, Default)]
pub struct PluginLoadCommandMessage {
    /// The filesystem path to the plugin shared library.
    pub path: String,
}

/// ABI-stable version of `PluginLoadCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct PluginLoadCommandMessageStabby {
    /// The filesystem path to the plugin shared library.
    pub path: stabby::string::String,
}

impl From<PluginLoadCommandMessage> for PluginLoadCommandMessageStabby {
    fn from(value: PluginLoadCommandMessage) -> Self {
        Self { path: value.path.into() }
    }
}

impl From<PluginLoadCommandMessageStabby> for PluginLoadCommandMessage {
    fn from(value: PluginLoadCommandMessageStabby) -> Self {
        Self { path: value.path.to_string() }
    }
}

impl TypedMessage for PluginLoadCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::PluginLoadCommandMessage");
}

impl TypedMessage for PluginLoadCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::PluginLoadCommandMessageStabby");
}

impl MessageTopic for PluginLoadCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for PluginLoadCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for PluginLoadCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
