use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by MacroPad services (StreamDeck, Loupedeck).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroPadMcpTools {
    /// Set brightness on StreamDeck devices.
    StreamDeckSetBrightness,
    /// Set brightness on Loupedeck devices.
    LoupedeckSetBrightness,
}

impl AsRef<str> for MacroPadMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::StreamDeckSetBrightness => "streamdeck_set_brightness",
            Self::LoupedeckSetBrightness => "loupedeck_set_brightness",
        }
    }
}

impl FromStr for MacroPadMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "streamdeck_set_brightness" => Ok(Self::StreamDeckSetBrightness),
            "loupedeck_set_brightness" => Ok(Self::LoupedeckSetBrightness),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for MacroPadMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
