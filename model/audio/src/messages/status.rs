use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_STATUS: &str = "service.audio.status";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct AudioDevice {
    /// Unique device identifier
    pub id: u32,
    /// Human-readable device name
    pub name: String,
    /// Whether this is the default/active device
    pub is_default: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct AudioStatusMessage {
    /// Current master volume (0.0 to 1.0, may exceed 1.0 if overdrive is enabled)
    pub volume: f32,
    /// Whether the audio is currently muted
    pub is_muted: bool,
    /// List of available output devices
    pub output_devices: Vec<AudioDevice>,
    /// List of available input devices
    pub input_devices: Vec<AudioDevice>,
    /// The currently active output device
    pub active_device: Option<AudioDevice>,
}

impl AudioStatusMessage {
    pub fn new(volume: f32, is_muted: bool, output_devices: Vec<AudioDevice>, input_devices: Vec<AudioDevice>, active_device: Option<AudioDevice>) -> Self {
        Self {
            volume,
            is_muted,
            output_devices,
            input_devices,
            active_device,
        }
    }
}

impl MessageTopic for AudioStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}
