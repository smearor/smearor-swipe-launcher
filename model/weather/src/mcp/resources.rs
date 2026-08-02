use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the weather service.
#[derive(Clone, Copy, Debug)]
pub enum WeatherMcpResources {
    /// Current weather at the configured or queried location.
    /// Supports optional query parameters `lat` and `lon` to override the configured coordinates.
    NowAtCurrentLocation(Option<(f64, f64)>),
}

impl WeatherMcpResources {
    pub(crate) fn parse_coordinates_from_uri(uri: &str) -> Option<(f64, f64)> {
        let base = "weather://now_at_current_location";
        let remainder = uri.strip_prefix(base)?;
        let query = remainder.strip_prefix('?').unwrap_or(remainder);
        if query == remainder || query.is_empty() {
            return None;
        }

        let mut latitude = None;
        let mut longitude = None;
        for part in query.split('&') {
            let (key, value) = part.split_once('=')?;
            let number = value.parse::<f64>().ok()?;
            match key {
                "lat" => latitude = Some(number),
                "lon" => longitude = Some(number),
                _ => {}
            }
        }

        Some((latitude?, longitude?))
    }
}

impl AsRef<str> for WeatherMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::NowAtCurrentLocation(_) => "weather://now_at_current_location",
        }
    }
}

impl FromStr for WeatherMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri.split('?').next().unwrap_or(uri) {
            "weather://now_at_current_location" => Ok(Self::NowAtCurrentLocation(Self::parse_coordinates_from_uri(uri))),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for WeatherMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
