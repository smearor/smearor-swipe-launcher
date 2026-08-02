use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMcpResources {
    Status,
    Volume,
    Muted,
    ActiveSink,
    Sinks,
}

impl AsRef<str> for AudioMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Status => "audio://status",
            Self::Volume => "audio://volume",
            Self::Muted => "audio://muted",
            Self::ActiveSink => "audio://active_sink",
            Self::Sinks => "audio://sinks",
        }
    }
}

impl FromStr for AudioMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "audio://status" => Ok(Self::Status),
            "audio://volume" => Ok(Self::Volume),
            "audio://muted" => Ok(Self::Muted),
            "audio://active_sink" => Ok(Self::ActiveSink),
            "audio://sinks" => Ok(Self::Sinks),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for AudioMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
