/// WMO weather interpretation code.
///
/// Represents the weather condition codes defined by the World Meteorological
/// Organization, as returned by the Open-Meteo API.
///
/// See <https://open-meteo.com/en/docs> for the full code reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeatherCode {
    /// Clear sky (WMO code 0).
    ClearSky,
    /// Mainly clear (WMO code 1).
    MainlyClear,
    /// Partly cloudy (WMO code 2).
    PartlyCloudy,
    /// Overcast (WMO code 3).
    Overcast,
    /// Fog (WMO code 45).
    Fog,
    /// Depositing rime fog (WMO code 48).
    DepositingRimeFog,
    /// Light drizzle (WMO code 51).
    LightDrizzle,
    /// Moderate drizzle (WMO code 53).
    ModerateDrizzle,
    /// Dense drizzle (WMO code 55).
    DenseDrizzle,
    /// Light freezing drizzle (WMO code 56).
    LightFreezingDrizzle,
    /// Dense freezing drizzle (WMO code 57).
    DenseFreezingDrizzle,
    /// Slight rain (WMO code 61).
    SlightRain,
    /// Moderate rain (WMO code 63).
    ModerateRain,
    /// Heavy rain (WMO code 65).
    HeavyRain,
    /// Light freezing rain (WMO code 66).
    LightFreezingRain,
    /// Heavy freezing rain (WMO code 67).
    HeavyFreezingRain,
    /// Slight snow fall (WMO code 71).
    SlightSnowFall,
    /// Moderate snow fall (WMO code 73).
    ModerateSnowFall,
    /// Heavy snow fall (WMO code 75).
    HeavySnowFall,
    /// Snow grains (WMO code 77).
    SnowGrains,
    /// Slight rain showers (WMO code 80).
    SlightRainShowers,
    /// Moderate rain showers (WMO code 81).
    ModerateRainShowers,
    /// Violent rain showers (WMO code 82).
    ViolentRainShowers,
    /// Slight snow showers (WMO code 85).
    SlightSnowShowers,
    /// Heavy snow showers (WMO code 86).
    HeavySnowShowers,
    /// Thunderstorm (WMO code 95).
    Thunderstorm,
    /// Thunderstorm with slight hail (WMO code 96).
    ThunderstormWithSlightHail,
    /// Thunderstorm with heavy hail (WMO code 99).
    ThunderstormWithHeavyHail,
    /// Unknown or unsupported weather code.
    Unknown,
}

impl WeatherCode {
    /// Converts a raw WMO weather code (`u16`) into a `WeatherCode` variant.
    pub fn from_code(code: u16) -> Self {
        match code {
            0 => Self::ClearSky,
            1 => Self::MainlyClear,
            2 => Self::PartlyCloudy,
            3 => Self::Overcast,
            45 => Self::Fog,
            48 => Self::DepositingRimeFog,
            51 => Self::LightDrizzle,
            53 => Self::ModerateDrizzle,
            55 => Self::DenseDrizzle,
            56 => Self::LightFreezingDrizzle,
            57 => Self::DenseFreezingDrizzle,
            61 => Self::SlightRain,
            63 => Self::ModerateRain,
            65 => Self::HeavyRain,
            66 => Self::LightFreezingRain,
            67 => Self::HeavyFreezingRain,
            71 => Self::SlightSnowFall,
            73 => Self::ModerateSnowFall,
            75 => Self::HeavySnowFall,
            77 => Self::SnowGrains,
            80 => Self::SlightRainShowers,
            81 => Self::ModerateRainShowers,
            82 => Self::ViolentRainShowers,
            85 => Self::SlightSnowShowers,
            86 => Self::HeavySnowShowers,
            95 => Self::Thunderstorm,
            96 => Self::ThunderstormWithSlightHail,
            99 => Self::ThunderstormWithHeavyHail,
            _ => Self::Unknown,
        }
    }

    /// Returns a human-readable description of the weather condition.
    pub fn description(&self) -> &'static str {
        match self {
            Self::ClearSky => "Clear sky",
            Self::MainlyClear => "Mainly clear",
            Self::PartlyCloudy => "Partly cloudy",
            Self::Overcast => "Overcast",
            Self::Fog => "Fog",
            Self::DepositingRimeFog => "Depositing rime fog",
            Self::LightDrizzle => "Light drizzle",
            Self::ModerateDrizzle => "Moderate drizzle",
            Self::DenseDrizzle => "Dense drizzle",
            Self::LightFreezingDrizzle => "Light freezing drizzle",
            Self::DenseFreezingDrizzle => "Dense freezing drizzle",
            Self::SlightRain => "Slight rain",
            Self::ModerateRain => "Moderate rain",
            Self::HeavyRain => "Heavy rain",
            Self::LightFreezingRain => "Light freezing rain",
            Self::HeavyFreezingRain => "Heavy freezing rain",
            Self::SlightSnowFall => "Slight snow fall",
            Self::ModerateSnowFall => "Moderate snow fall",
            Self::HeavySnowFall => "Heavy snow fall",
            Self::SnowGrains => "Snow grains",
            Self::SlightRainShowers => "Slight rain showers",
            Self::ModerateRainShowers => "Moderate rain showers",
            Self::ViolentRainShowers => "Violent rain showers",
            Self::SlightSnowShowers => "Slight snow showers",
            Self::HeavySnowShowers => "Heavy snow showers",
            Self::Thunderstorm => "Thunderstorm",
            Self::ThunderstormWithSlightHail => "Thunderstorm with slight hail",
            Self::ThunderstormWithHeavyHail => "Thunderstorm with heavy hail",
            Self::Unknown => "Unknown",
        }
    }

    /// Returns the Nerd Font icon name for daytime conditions.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::ClearSky => "nf-weather-day_sunny",
            Self::MainlyClear => "nf-weather-day_sunny_overcast",
            Self::PartlyCloudy => "nf-weather-day_cloudy",
            Self::Overcast => "nf-weather-cloudy",
            Self::Fog | Self::DepositingRimeFog => "nf-weather-fog",
            Self::LightDrizzle | Self::LightFreezingDrizzle => "nf-weather-rain_mix",
            Self::ModerateDrizzle => "nf-weather-rain",
            Self::DenseDrizzle | Self::DenseFreezingDrizzle => "nf-weather-rain_wind",
            Self::SlightRain | Self::LightFreezingRain => "nf-weather-rain",
            Self::ModerateRain => "nf-weather-rain",
            Self::HeavyRain | Self::HeavyFreezingRain => "nf-weather-rain_wind",
            Self::SlightSnowFall | Self::SnowGrains | Self::SlightSnowShowers => "nf-weather-snow",
            Self::ModerateSnowFall => "nf-weather-snow",
            Self::HeavySnowFall | Self::HeavySnowShowers => "nf-weather-snow_wind",
            Self::SlightRainShowers | Self::ModerateRainShowers => "nf-weather-showers",
            Self::ViolentRainShowers => "nf-weather-showers_wind",
            Self::Thunderstorm | Self::ThunderstormWithSlightHail | Self::ThunderstormWithHeavyHail => "nf-weather-storm_showers",
            Self::Unknown => "nf-weather-alien",
        }
    }

    /// Returns the Nerd Font icon name, considering whether it is day or night.
    pub fn icon_day_night(&self, is_day: bool) -> &'static str {
        if is_day {
            return self.icon();
        }
        match self {
            Self::ClearSky => "nf-weather-night_clear",
            Self::MainlyClear => "nf-weather-night_clear",
            Self::PartlyCloudy => "nf-weather-night_partly_cloudy",
            Self::Overcast => "nf-weather-night_cloudy",
            Self::Fog | Self::DepositingRimeFog => "nf-weather-night_fog",
            Self::LightDrizzle | Self::LightFreezingDrizzle => "nf-weather-night_alt_rain_mix",
            Self::ModerateDrizzle => "nf-weather-night_alt_rain",
            Self::DenseDrizzle | Self::DenseFreezingDrizzle => "nf-weather-night_alt_rain_mix",
            Self::SlightRain | Self::LightFreezingRain => "nf-weather-night_alt_rain",
            Self::ModerateRain => "nf-weather-night_alt_rain",
            Self::HeavyRain | Self::HeavyFreezingRain => "nf-weather-night_alt_rain",
            Self::SlightSnowFall | Self::SnowGrains | Self::SlightSnowShowers => "nf-weather-night_alt_snow",
            Self::ModerateSnowFall => "nf-weather-night_alt_snow",
            Self::HeavySnowFall | Self::HeavySnowShowers => "nf-weather-night_alt_snow",
            Self::SlightRainShowers | Self::ModerateRainShowers => "nf-weather-night_alt_showers",
            Self::ViolentRainShowers => "nf-weather-night_alt_showers",
            Self::Thunderstorm | Self::ThunderstormWithSlightHail | Self::ThunderstormWithHeavyHail => "nf-weather-night_alt_storm_showers",
            Self::Unknown => "nf-weather-alien",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WeatherCode;

    #[test]
    fn description_for_clear_sky() {
        assert_eq!(WeatherCode::from_code(0).description(), "Clear sky");
    }

    #[test]
    fn description_for_thunderstorm() {
        assert_eq!(WeatherCode::from_code(95).description(), "Thunderstorm");
    }

    #[test]
    fn description_for_unknown_code() {
        assert_eq!(WeatherCode::from_code(999).description(), "Unknown");
    }

    #[test]
    fn icon_for_clear_sky() {
        assert_eq!(WeatherCode::from_code(0).icon(), "nf-weather-day_sunny");
    }

    #[test]
    fn icon_for_overcast() {
        assert_eq!(WeatherCode::from_code(3).icon(), "nf-weather-cloudy");
    }
}
