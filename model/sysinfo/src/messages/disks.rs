use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::TOPIC_DISKS;

/// Usage information for a single mount point.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug)]
pub struct DiskUsage {
    /// Mount point path.
    pub mount_point: stabby::string::String,
    /// Usage percentage (0.0 - 100.0).
    pub usage: f32,
    /// Total capacity in bytes.
    pub total: u64,
    /// Used space in bytes.
    pub used: u64,
    /// Available space in bytes.
    pub available: u64,
}

/// Status message for disk metrics.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct DisksStatusMessage {
    /// Per-mount usage information.
    pub mounts: stabby::vec::Vec<DiskUsage>,
    /// Aggregate read throughput in bytes per second.
    pub read_bytes_per_second: u64,
    /// Aggregate write throughput in bytes per second.
    pub write_bytes_per_second: u64,
}

impl DisksStatusMessage {
    /// Creates a new disks status message.
    pub fn new(mounts: stabby::vec::Vec<DiskUsage>, read_bytes_per_second: u64, write_bytes_per_second: u64) -> Self {
        Self {
            mounts,
            read_bytes_per_second,
            write_bytes_per_second,
        }
    }
}

impl TypedMessage for DisksStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_sysinfo_model::DisksStatusMessage");
}

impl MessageTopic for DisksStatusMessage {
    fn topic() -> &'static str {
        TOPIC_DISKS
    }
}

impl SharedMessage for DisksStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_DISKS
    }
}
