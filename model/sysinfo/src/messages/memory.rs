use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_MEMORY;

/// Status message for memory metrics.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct MemoryStatusMessage {
    /// Memory usage in percent (0.0 - 100.0).
    pub memory_usage: f32,
    /// Total physical memory in bytes.
    pub memory_total: u64,
    /// Used physical memory in bytes.
    pub memory_used: u64,
    /// Available physical memory in bytes (without cache).
    pub memory_available: u64,
}

impl MemoryStatusMessage {
    /// Creates a new memory status message.
    pub fn new(memory_usage: f32, memory_total: u64, memory_used: u64, memory_available: u64) -> Self {
        Self {
            memory_usage,
            memory_total,
            memory_used,
            memory_available,
        }
    }
}

impl TypedMessage for MemoryStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_sysinfo_model::MemoryStatusMessage");
}

impl MessageTopic for MemoryStatusMessage {
    fn topic() -> &'static str {
        TOPIC_MEMORY
    }
}

impl SharedMessage for MemoryStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_MEMORY
    }
}
