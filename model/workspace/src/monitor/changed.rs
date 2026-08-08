use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::monitor::change_type::MonitorChangeType;

/// Topic for monitor change events broadcast by compositor services.
pub const TOPIC_MONITOR_CHANGED: &str = "compositor.monitor.changed";

/// Event broadcast when a monitor is connected or disconnected.
///
/// Launcher instances use this to re-evaluate monitor mappings and rebuild areas.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MonitorChangedEvent {
    /// The monitor index (0-based, matching GDK display order).
    pub monitor_index: u32,
    /// The connector name of the monitor (e.g. "HDMI-A-1", "eDP-1").
    pub connector_name: stabby::string::String,
    /// Whether the monitor was connected or disconnected.
    pub change_type: MonitorChangeType,
}

impl TypedMessage for MonitorChangedEvent {
    const TYPE_ID: u64 = generate_type_id("smearor_model_compositor::MonitorChangedEvent");
}

impl MessageTopic for MonitorChangedEvent {
    fn topic() -> &'static str {
        TOPIC_MONITOR_CHANGED
    }
}

impl SharedMessage for MonitorChangedEvent {
    fn topic(&self) -> &'static str {
        TOPIC_MONITOR_CHANGED
    }
}
