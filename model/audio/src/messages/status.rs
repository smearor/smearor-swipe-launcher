use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

pub const TOPIC_STATUS: &str = "service.audio.status";

/// Information about an audio device.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq)]
pub struct AudioDevice {
    /// Unique device identifier
    pub id: u32,
    /// Human-readable device name
    pub name: stabby::string::String,
    /// Whether this is the default/active device
    pub is_default: bool,
}

/// Status message broadcast by the audio service to all widgets.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq)]
pub struct AudioStatusMessage {
    /// Current master volume (0.0 to 1.0, may exceed 1.0 if overdrive is enabled)
    pub volume: f32,
    /// Whether the audio is currently muted
    pub is_muted: bool,
    /// List of available output devices
    pub output_devices: stabby::vec::Vec<AudioDevice>,
    /// List of available input devices
    pub input_devices: stabby::vec::Vec<AudioDevice>,
    /// The currently active output device
    pub active_device: stabby::option::Option<AudioDevice>,
}

impl AudioStatusMessage {
    pub fn new(
        volume: f32,
        is_muted: bool,
        output_devices: stabby::vec::Vec<AudioDevice>,
        input_devices: stabby::vec::Vec<AudioDevice>,
        active_device: stabby::option::Option<AudioDevice>,
    ) -> Self {
        Self {
            volume,
            is_muted,
            output_devices,
            input_devices,
            active_device,
        }
    }
}

impl TypedMessage for AudioStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_audio_model::AudioStatusMessage");
}

impl MessageTopic for AudioStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for AudioStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}
