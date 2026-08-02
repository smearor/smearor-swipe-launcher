use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the personalization service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonalizationMcpTools {
    /// Get the current geographic location.
    GetCurrentLocation,
    /// Get the current timezone.
    GetTimezone,
    /// Get the current locale.
    GetLocale,
    /// Get the full personalization state.
    GetPersonalization,
    /// Set the current geographic location.
    SetCurrentLocation,
    /// Set the locale.
    SetLocale,
    /// Refresh personalization state and clear runtime overrides.
    RefreshPersonalization,
}

impl AsRef<str> for PersonalizationMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::GetCurrentLocation => "get_current_location",
            Self::GetTimezone => "get_timezone",
            Self::GetLocale => "get_locale",
            Self::GetPersonalization => "get_personalization",
            Self::SetCurrentLocation => "set_current_location",
            Self::SetLocale => "set_locale",
            Self::RefreshPersonalization => "refresh_personalization",
        }
    }
}

impl FromStr for PersonalizationMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "get_current_location" => Ok(Self::GetCurrentLocation),
            "get_timezone" => Ok(Self::GetTimezone),
            "get_locale" => Ok(Self::GetLocale),
            "get_personalization" => Ok(Self::GetPersonalization),
            "set_current_location" => Ok(Self::SetCurrentLocation),
            "set_locale" => Ok(Self::SetLocale),
            "refresh_personalization" => Ok(Self::RefreshPersonalization),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for PersonalizationMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
