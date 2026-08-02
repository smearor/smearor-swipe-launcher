use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::topics::TOPIC_MACROPAD_CONNECTION;

/// Connection status of a MacroPad device.
///
/// Broadcast by MacroPad services when a device is connected or disconnected.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MacroPadConnectionStatus {
    /// Unique identifier for the device (serial number or composite ID).
    pub device_id: stabby::string::String,
    /// Instance ID assigned to the device.
    pub instance_id: stabby::string::String,
    /// Device type identifier (e.g. "streamdeck_original_v2", "streamdeck_mk2", "loupedeck_ct").
    pub device_type: stabby::string::String,
    /// Driver/service that manages this device (e.g. "streamdeck", "loupedeck").
    pub driver: stabby::string::String,
    /// Number of keys on the device.
    pub key_count: u8,
    /// Number of columns in the device's button grid.
    ///
    /// Used by the host to map 2D span group positions to physical button
    /// indices. For devices with a single row, this equals `key_count`.
    pub key_columns: u8,
    /// Key resolution width in pixels.
    pub key_width: u32,
    /// Key resolution height in pixels.
    pub key_height: u32,
    /// True if connected, false if disconnected.
    pub connected: bool,
}

impl MacroPadConnectionStatus {
    /// Create a new connection status message.
    pub fn new(
        device_id: &str,
        instance_id: &str,
        device_type: &str,
        driver: &str,
        key_count: u8,
        key_columns: u8,
        key_width: u32,
        key_height: u32,
        connected: bool,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            instance_id: instance_id.into(),
            device_type: device_type.into(),
            driver: driver.into(),
            key_count,
            key_columns,
            key_width,
            key_height,
            connected,
        }
    }
}

impl TypedMessage for MacroPadConnectionStatus {
    const TYPE_ID: u64 = generate_type_id("smearor_model_macropad::MacroPadConnectionStatus");
}

impl MessageTopic for MacroPadConnectionStatus {
    fn topic() -> &'static str {
        TOPIC_MACROPAD_CONNECTION
    }
}

impl SharedMessage for MacroPadConnectionStatus {
    fn topic(&self) -> &'static str {
        TOPIC_MACROPAD_CONNECTION
    }
}
