use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::topics::TOPIC_CORE_INSTANCE_RELOAD;

/// Message to hot-reload a running instance (stop + load with same ID).
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceReloadMessage {
    /// Unique identifier of the instance to reload.
    pub instance_id: stabby::string::String,
    /// File system path to the TOML config file for the reloaded instance.
    pub config_path: stabby::string::String,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}

impl InstanceReloadMessage {
    /// Create a new instance reload message.
    pub fn new(instance_id: &str, config_path: &str, response_topic: &str) -> Self {
        Self {
            instance_id: instance_id.into(),
            config_path: config_path.into(),
            response_topic: response_topic.into(),
        }
    }
}

impl TypedMessage for InstanceReloadMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_instance_control::InstanceReloadMessage");
}

impl MessageTopic for InstanceReloadMessage {
    fn topic() -> &'static str {
        TOPIC_CORE_INSTANCE_RELOAD
    }
}

impl SharedMessage for InstanceReloadMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CORE_INSTANCE_RELOAD
    }
}
