use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use super::kill::TOPIC_CTL;

/// Unloads a Hyprland plugin by name.
#[derive(Clone, Debug, Default)]
pub struct PluginUnloadCommandMessage {
    /// The name of the plugin to unload.
    pub name: String,
}

/// ABI-stable version of `PluginUnloadCommandMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct PluginUnloadCommandMessageStabby {
    /// The name of the plugin to unload.
    pub name: stabby::string::String,
}

impl From<PluginUnloadCommandMessage> for PluginUnloadCommandMessageStabby {
    fn from(value: PluginUnloadCommandMessage) -> Self {
        Self { name: value.name.into() }
    }
}

impl From<PluginUnloadCommandMessageStabby> for PluginUnloadCommandMessage {
    fn from(value: PluginUnloadCommandMessageStabby) -> Self {
        Self { name: value.name.to_string() }
    }
}

impl TypedMessage for PluginUnloadCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::PluginUnloadCommandMessage");
}

impl TypedMessage for PluginUnloadCommandMessageStabby {
    const TYPE_ID: u64 = generate_type_id("smearor_hyprland_model::PluginUnloadCommandMessageStabby");
}

impl MessageTopic for PluginUnloadCommandMessage {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl MessageTopic for PluginUnloadCommandMessageStabby {
    fn topic() -> &'static str {
        TOPIC_CTL
    }
}

impl SharedMessage for PluginUnloadCommandMessageStabby {
    fn topic(&self) -> &'static str {
        TOPIC_CTL
    }
}
