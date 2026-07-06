/// Returns a human-readable description for a WMO weather interpretation code.
pub fn weather_code_description(code: u16) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Fog",
        48 => "Depositing rime fog",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense drizzle",
        56 => "Light freezing drizzle",
        57 => "Dense freezing drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        66 => "Light freezing rain",
        67 => "Heavy freezing rain",
        71 => "Slight snow fall",
        73 => "Moderate snow fall",
        75 => "Heavy snow fall",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 => "Thunderstorm with slight hail",
        99 => "Thunderstorm with heavy hail",
        _ => "Unknown",
    }
}

/// Returns the Nerd Font icon name for a WMO weather interpretation code.
pub fn weather_code_icon(code: u16) -> &'static str {
    match code {
        0 => "\u{e30d}",            // nf-weather-day_sunny
        1 => "\u{e30c}",            // nf-weather-day_sunny_overcast
        2 => "\u{e302}",            // nf-weather-day_cloudy
        3 => "\u{e312}",            // nf-weather-cloudy
        45 | 48 => "\u{e313}",      // nf-weather_fog
        51 | 56 => "\u{e316}",      // nf-weather_rain_mix
        53 => "\u{e318}",           // nf-weather_rain
        55 | 57 => "\u{e317}",      // nf-weather_rain_wind
        61 | 66 => "\u{e318}",      // nf-weather_rain
        63 => "\u{e318}",           // nf-weather_rain
        65 | 67 => "\u{ef1d}",      // nf-weather_rain_wind
        71 | 77 | 85 => "\u{e31a}", // nf-weather_snow
        73 => "\u{e31a}",           // nf-weather_snow
        75 | 86 => "\u{e35e}",      // nf-weather_snow_wind
        80 | 81 => "\u{e319}",      // nf-weather_showers
        82 => "\u{f01a}",           // nf-weather_showers_wind
        95 | 96 | 99 => "\u{e337}", // nf-weather_storm_showers
        _ => "\u{e36e}",            // nf-weather-alien
    }
}

/// Returns the Nerd Font icon name for a WMO weather interpretation code,
/// considering whether it is day or night.
pub fn weather_code_icon_day_night(code: u16, is_day: bool) -> &'static str {
    if !is_day {
        return match code {
            0 => "\u{f0594}",           // nf-weather-night_clear
            1 => "\u{f0f31}",           // nf-weather-night_alt_clouds
            2 => "\u{f0f31}",           // nf-weather-night_alt_cloudy
            3 => "\u{f0590}",           // nf-weather-cloudy
            45 | 48 => "\u{f0591}",     // nf-weather_fog
            51 | 56 => "\u{e323}",      // nf-weather-night_alt_rain_mix
            53 => "\u{e325}",           // nf-weather-night_alt_rain
            55 | 57 => "\u{e323}",      // nf-weather-night_alt_rain_mix
            61 | 66 => "\u{e325}",      // nf-weather-night_alt_rain
            63 => "\u{e325}",           // nf-weather-night_alt_rain
            65 | 67 => "\u{e325}",      // nf-weather-night_alt_rain
            71 | 77 | 85 => "\u{e327}", // nf-weather-night_alt_snow
            73 => "\u{e327}",           // nf-weather-night_alt_snow
            75 | 86 => "\u{e327}",      // nf-weather-night_alt_snow
            80 | 81 => "\u{e326}",      // nf-weather-night_alt_showers
            82 => "\u{e326}",           // nf-weather-night_alt_showers
            95 | 96 | 99 => "\u{e329}", // nf-weather-night_alt_storm_showers
            _ => "\u{e36e}",            // nf-weather-alien
        };
    }
    weather_code_icon(code)
}

#[cfg(test)]
mod tests {
    use super::weather_code_description;
    use super::weather_code_icon;

    #[test]
    fn description_for_clear_sky() {
        assert_eq!(weather_code_description(0), "Clear sky");
    }

    #[test]
    fn description_for_thunderstorm() {
        assert_eq!(weather_code_description(95), "Thunderstorm");
    }

    #[test]
    fn description_for_unknown_code() {
        assert_eq!(weather_code_description(999), "Unknown");
    }

    #[test]
    fn icon_for_clear_sky() {
        assert_eq!(weather_code_icon(0), "\u{e30d}");
    }

    #[test]
    fn icon_for_overcast() {
        assert_eq!(weather_code_icon(3), "\u{e312}");
    }
}
