use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_CPU;

/// Status message for CPU metrics.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct CpuStatusMessage {
    /// CPU usage in percent (0.0 - 100.0).
    pub cpu_usage: f32,
    /// CPU temperature in degrees Celsius.
    pub cpu_temperature: stabby::option::Option<f32>,
}

impl CpuStatusMessage {
    /// Creates a new CPU status message.
    pub fn new(cpu_usage: f32, cpu_temperature: stabby::option::Option<f32>) -> Self {
        Self { cpu_usage, cpu_temperature }
    }
}

impl TypedMessage for CpuStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_sysinfo_model::CpuStatusMessage");
}

impl MessageTopic for CpuStatusMessage {
    fn topic() -> &'static str {
        TOPIC_CPU
    }
}

impl SharedMessage for CpuStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_CPU
    }
}
