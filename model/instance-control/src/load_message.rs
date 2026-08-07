use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::instance_type::InstanceType;
use crate::topics::TOPIC_CORE_INSTANCE_LOAD;

/// Message to dynamically load a new launcher instance from a config file.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceLoadMessage {
    /// Unique identifier for the new instance.
    pub instance_id: stabby::string::String,
    /// File system path to the TOML config file for this instance.
    pub config_path: stabby::string::String,
    /// Whether to create a GTK window (Gtk) or run headless (Headless) or web (Web).
    pub instance_type: InstanceType,
    /// Whether to persist this instance to the state file so it survives restarts.
    /// Set to `true` for config-file instances discovered at startup.
    /// Set to `false` for transient instances loaded at runtime (default).
    pub persist: bool,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}

impl InstanceLoadMessage {
    /// Create a new instance load message.
    pub fn new(instance_id: &str, config_path: &str, instance_type: InstanceType, persist: bool, response_topic: &str) -> Self {
        Self {
            instance_id: instance_id.into(),
            config_path: config_path.into(),
            instance_type,
            persist,
            response_topic: response_topic.into(),
        }
    }
}

impl TypedMessage for InstanceLoadMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_instance_control::InstanceLoadMessage");
}

impl MessageTopic for InstanceLoadMessage {
    fn topic() -> &'static str {
        TOPIC_CORE_INSTANCE_LOAD
    }
}

impl SharedMessage for InstanceLoadMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CORE_INSTANCE_LOAD
    }
}
