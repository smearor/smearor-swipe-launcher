use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::topics::TOPIC_CORE_INSTANCE_START;

/// Message to dynamically start a loaded (Ready) launcher instance.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceStartMessage {
    /// Unique identifier of the instance to start.
    pub instance_id: stabby::string::String,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}

impl InstanceStartMessage {
    /// Create a new instance start message.
    pub fn new(instance_id: &str, response_topic: &str) -> Self {
        Self {
            instance_id: instance_id.into(),
            response_topic: response_topic.into(),
        }
    }
}

impl TypedMessage for InstanceStartMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_instance_control::InstanceStartMessage");
}

impl MessageTopic for InstanceStartMessage {
    fn topic() -> &'static str {
        TOPIC_CORE_INSTANCE_START
    }
}

impl SharedMessage for InstanceStartMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CORE_INSTANCE_START
    }
}
