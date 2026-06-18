use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;
use stabby::option::Option;

pub const TOPIC_COMMAND: &str = "service.audio.command";

/// Actions that can be sent to the audio service.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default)]
pub enum AudioCommandAction {
    #[default]
    /// Increase the volume by a relative amount
    VolumeUp,
    /// Decrease the volume by a relative amount
    VolumeDown,
    /// Set the volume to an absolute value (0.0 - 1.0)
    SetVolume,
    /// Toggle the mute state
    ToggleMute,
    /// Mute the audio
    Mute,
    /// Unmute the audio
    Unmute,
    /// Switch to the next output device
    NextDevice,
    /// Switch to the previous output device
    PreviousDevice,
}

/// Command message sent from the audio widget to the audio service.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct AudioCommandMessage {
    /// The action to execute
    pub action: AudioCommandAction,
    /// Optional absolute volume value (0.0 to 1.0, used with SetVolume)
    pub volume: Option<f32>,
    /// Optional device identifier for device switching
    pub device_id: Option<u32>,
}

impl AudioCommandMessage {
    pub fn new(action: AudioCommandAction, volume: Option<f32>, device_id: Option<u32>) -> Self {
        Self { action, volume, device_id }
    }

    pub fn volume_up() -> Self {
        Self::new(AudioCommandAction::VolumeUp, Option::None(), Option::None())
    }

    pub fn volume_down() -> Self {
        Self::new(AudioCommandAction::VolumeDown, Option::None(), Option::None())
    }

    pub fn set_volume(volume: f32) -> Self {
        Self::new(AudioCommandAction::SetVolume, Option::Some(volume), Option::None())
    }

    pub fn toggle_mute() -> Self {
        Self::new(AudioCommandAction::ToggleMute, Option::None(), Option::None())
    }

    pub fn mute() -> Self {
        Self::new(AudioCommandAction::Mute, Option::None(), Option::None())
    }

    pub fn unmute() -> Self {
        Self::new(AudioCommandAction::Unmute, Option::None(), Option::None())
    }

    pub fn next_device() -> Self {
        Self::new(AudioCommandAction::NextDevice, Option::None(), Option::None())
    }

    pub fn previous_device() -> Self {
        Self::new(AudioCommandAction::PreviousDevice, Option::None(), Option::None())
    }
}

impl TypedMessage for AudioCommandMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_audio_model::AudioCommandMessage");
}

impl MessageTopic for AudioCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}

impl SharedMessage for AudioCommandMessage {
    fn topic(&self) -> &'static str {
        TOPIC_COMMAND
    }
}
