use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::topics::TOPIC_CORE_INSTANCE_STOP;

/// Message to dynamically stop and remove a running launcher instance.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceStopMessage {
    /// Unique identifier of the instance to stop.
    pub instance_id: stabby::string::String,
    /// Optional broker topic to send the result response to.
    /// Empty string means no response is expected.
    pub response_topic: stabby::string::String,
}

impl InstanceStopMessage {
    /// Create a new instance stop message.
    pub fn new(instance_id: &str, response_topic: &str) -> Self {
        Self {
            instance_id: instance_id.into(),
            response_topic: response_topic.into(),
        }
    }
}

impl TypedMessage for InstanceStopMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_instance_control::InstanceStopMessage");
}

impl MessageTopic for InstanceStopMessage {
    fn topic() -> &'static str {
        TOPIC_CORE_INSTANCE_STOP
    }
}

impl SharedMessage for InstanceStopMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CORE_INSTANCE_STOP
    }
}
