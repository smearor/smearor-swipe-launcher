use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;

pub const TOPIC_COMMAND: &str = "service.audio.command";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
        Self::new(AudioCommandAction::VolumeUp, None, None)
    }

    pub fn volume_down() -> Self {
        Self::new(AudioCommandAction::VolumeDown, None, None)
    }

    pub fn set_volume(volume: f32) -> Self {
        Self::new(AudioCommandAction::SetVolume, Some(volume), None)
    }

    pub fn toggle_mute() -> Self {
        Self::new(AudioCommandAction::ToggleMute, None, None)
    }

    pub fn mute() -> Self {
        Self::new(AudioCommandAction::Mute, None, None)
    }

    pub fn unmute() -> Self {
        Self::new(AudioCommandAction::Unmute, None, None)
    }

    pub fn next_device() -> Self {
        Self::new(AudioCommandAction::NextDevice, None, None)
    }

    pub fn previous_device() -> Self {
        Self::new(AudioCommandAction::PreviousDevice, None, None)
    }
}

impl MessageTopic for AudioCommandMessage {
    fn topic() -> &'static str {
        TOPIC_COMMAND
    }
}
