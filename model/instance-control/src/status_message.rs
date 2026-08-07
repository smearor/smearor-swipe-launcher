use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::lifecycle_event::LauncherInstanceLifecycle;
use crate::topics::TOPIC_CORE_INSTANCE_STATUS;

/// Status message broadcast when an instance is loaded or stopped.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceStatusMessage {
    /// The instance ID that changed.
    pub instance_id: stabby::string::String,
    /// The lifecycle state of the instance.
    pub event: LauncherInstanceLifecycle,
}

impl InstanceStatusMessage {
    /// Create a new instance status message.
    pub fn new(instance_id: &str, event: LauncherInstanceLifecycle) -> Self {
        Self {
            instance_id: instance_id.into(),
            event,
        }
    }
}

impl TypedMessage for InstanceStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_model_instance_control::InstanceStatusMessage");
}

impl MessageTopic for InstanceStatusMessage {
    fn topic() -> &'static str {
        TOPIC_CORE_INSTANCE_STATUS
    }
}

impl SharedMessage for InstanceStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CORE_INSTANCE_STATUS
    }
}
