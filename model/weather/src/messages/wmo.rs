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
        0 => "\u{f00d}",            // nf-weather-day_sunny
        1 => "\u{f00c}",            // nf-weather-day_sunny_overcast
        2 => "\u{f002}",            // nf-weather-day_cloudy
        3 => "\u{f013}",            // nf-weather-cloudy
        45 | 48 => "\u{f014}",      // nf-weather_fog
        51 | 56 => "\u{f017}",      // nf-weather_rain_mix
        53 => "\u{f019}",           // nf-weather_rain
        55 | 57 => "\u{f015}",      // nf-weather_rain_wind
        61 | 66 => "\u{f019}",      // nf-weather_rain
        63 => "\u{f019}",           // nf-weather_rain
        65 | 67 => "\u{f015}",      // nf-weather_rain_wind
        71 | 77 | 85 => "\u{f01b}", // nf-weather_snow
        73 => "\u{f01b}",           // nf-weather_snow
        75 | 86 => "\u{f064}",      // nf-weather_snow_wind
        80 | 81 => "\u{f01a}",      // nf-weather_showers
        82 => "\u{f01a}",           // nf-weather_showers_wind
        95 | 96 | 99 => "\u{f016}", // nf-weather_storm_showers
        _ => "\u{f07b}",            // nf-weather-alien
    }
}

/// Returns the Nerd Font icon name for a WMO weather interpretation code,
/// considering whether it is day or night.
pub fn weather_code_icon_day_night(code: u16, is_day: bool) -> &'static str {
    if !is_day {
        return match code {
            0 => "\u{f02e}",            // nf-weather-night_clear
            1 => "\u{f083}",            // nf-weather-night_alt_clouds
            2 => "\u{f086}",            // nf-weather-night_alt_cloudy
            3 => "\u{f013}",            // nf-weather-cloudy
            45 | 48 => "\u{f014}",      // nf-weather_fog
            51 | 56 => "\u{f0b6}",      // nf-weather-night_alt_rain_mix
            53 => "\u{f029}",           // nf-weather-night_alt_rain
            55 | 57 => "\u{f0b6}",      // nf-weather-night_alt_rain_mix
            61 | 66 => "\u{f029}",      // nf-weather-night_alt_rain
            63 => "\u{f029}",           // nf-weather-night_alt_rain
            65 | 67 => "\u{f029}",      // nf-weather-night_alt_rain
            71 | 77 | 85 => "\u{f02a}", // nf-weather-night_alt_snow
            73 => "\u{f02a}",           // nf-weather-night_alt_snow
            75 | 86 => "\u{f02a}",      // nf-weather-night_alt_snow
            80 | 81 => "\u{f037}",      // nf-weather-night_alt_showers
            82 => "\u{f037}",           // nf-weather-night_alt_showers
            95 | 96 | 99 => "\u{f033}", // nf-weather-night_alt_storm_showers
            _ => "\u{f07b}",            // nf-weather-alien
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
        assert_eq!(weather_code_icon(0), "\u{f00d}");
    }

    #[test]
    fn icon_for_overcast() {
        assert_eq!(weather_code_icon(3), "\u{f013}");
    }
}
