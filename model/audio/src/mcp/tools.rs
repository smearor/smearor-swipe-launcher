use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMcpTools {
    VolumeUp,
    VolumeDown,
    SetVolume,
    ToggleMute,
    Mute,
    Unmute,
    NextDevice,
    PreviousDevice,
    RefreshStatus,
}

impl AsRef<str> for AudioMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::VolumeUp => "audio_volume_up",
            Self::VolumeDown => "audio_volume_down",
            Self::SetVolume => "audio_set_volume",
            Self::ToggleMute => "audio_toggle_mute",
            Self::Mute => "audio_mute",
            Self::Unmute => "audio_unmute",
            Self::NextDevice => "audio_next_device",
            Self::PreviousDevice => "audio_previous_device",
            Self::RefreshStatus => "audio_refresh_status",
        }
    }
}

impl FromStr for AudioMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "audio_volume_up" => Ok(Self::VolumeUp),
            "audio_volume_down" => Ok(Self::VolumeDown),
            "audio_set_volume" => Ok(Self::SetVolume),
            "audio_toggle_mute" => Ok(Self::ToggleMute),
            "audio_mute" => Ok(Self::Mute),
            "audio_unmute" => Ok(Self::Unmute),
            "audio_next_device" => Ok(Self::NextDevice),
            "audio_previous_device" => Ok(Self::PreviousDevice),
            "audio_refresh_status" => Ok(Self::RefreshStatus),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for AudioMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
