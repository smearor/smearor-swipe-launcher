use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the MPRIS service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MprisMcpResources {
    /// Full playback status including player info, metadata, and position.
    Status,
    /// List of all available MPRIS players.
    Players,
    /// Compact playback status (has_player + playback_status).
    Playback,
    /// Current track metadata (title, artist, album, length, art_url).
    Metadata,
}

impl AsRef<str> for MprisMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Status => "mpris://status",
            Self::Players => "mpris://players",
            Self::Playback => "mpris://playback",
            Self::Metadata => "mpris://metadata",
        }
    }
}

impl FromStr for MprisMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "mpris://status" => Ok(Self::Status),
            "mpris://players" => Ok(Self::Players),
            "mpris://playback" => Ok(Self::Playback),
            "mpris://metadata" => Ok(Self::Metadata),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for MprisMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
