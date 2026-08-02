use smearor_model_mcp::UnknownPromptError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP prompts registered by the weather service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherMcpPrompts {
    /// Guide for querying weather data.
    WeatherQueryGuide,
    /// Guide for resolving weather locations.
    WeatherContextGuide,
}

impl AsRef<str> for WeatherMcpPrompts {
    fn as_ref(&self) -> &str {
        match self {
            Self::WeatherQueryGuide => "weather_query_guide",
            Self::WeatherContextGuide => "weather_context_guide",
        }
    }
}

impl FromStr for WeatherMcpPrompts {
    type Err = UnknownPromptError;

    fn from_str(prompt: &str) -> Result<Self, Self::Err> {
        match prompt {
            "weather_query_guide" => Ok(Self::WeatherQueryGuide),
            "weather_context_guide" => Ok(Self::WeatherContextGuide),
            _ => Err(UnknownPromptError::new(prompt)),
        }
    }
}

impl Display for WeatherMcpPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
