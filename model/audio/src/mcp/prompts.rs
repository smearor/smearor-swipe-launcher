use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the audio service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMcpPrompts {
    /// Guide for controlling audio volume, mute, and output devices.
    AudioControlGuide,
}

impl AsRef<str> for AudioMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::AudioControlGuide => "audio_control_guide",
        }
    }
}

impl FromStr for AudioMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "audio_control_guide" => Ok(Self::AudioControlGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for AudioMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
