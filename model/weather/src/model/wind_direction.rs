use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// 16-point compass direction for wind bearing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindDirection {
    /// North (0°).
    North,
    /// North-north-east (22.5°).
    NorthNorthEast,
    /// North-east (45°).
    NorthEast,
    /// East-north-east (67.5°).
    EastNorthEast,
    /// East (90°).
    East,
    /// East-south-east (112.5°).
    EastSouthEast,
    /// South-east (135°).
    SouthEast,
    /// South-south-east (157.5°).
    SouthSouthEast,
    /// South (180°).
    South,
    /// South-south-west (202.5°).
    SouthSouthWest,
    /// South-west (225°).
    SouthWest,
    /// West-south-west (247.5°).
    WestSouthWest,
    /// West (270°).
    West,
    /// West-north-west (292.5°).
    WestNorthWest,
    /// North-west (315°).
    NorthWest,
    /// North-north-west (337.5°).
    NorthNorthWest,
}

impl From<f32> for WindDirection {
    fn from(degrees: f32) -> Self {
        let index = ((degrees + 11.25) / 22.5) as usize % 16;
        match index {
            0 => Self::North,
            1 => Self::NorthNorthEast,
            2 => Self::NorthEast,
            3 => Self::EastNorthEast,
            4 => Self::East,
            5 => Self::EastSouthEast,
            6 => Self::SouthEast,
            7 => Self::SouthSouthEast,
            8 => Self::South,
            9 => Self::SouthSouthWest,
            10 => Self::SouthWest,
            11 => Self::WestSouthWest,
            12 => Self::West,
            13 => Self::WestNorthWest,
            14 => Self::NorthWest,
            _ => Self::NorthNorthWest,
        }
    }
}

impl AsRef<str> for WindDirection {
    fn as_ref(&self) -> &str {
        match self {
            Self::North => "Nord",
            Self::NorthNorthEast => "Nord-Nord-Ost",
            Self::NorthEast => "Nord-Ost",
            Self::EastNorthEast => "Ost-Nord-Ost",
            Self::East => "Ost",
            Self::EastSouthEast => "Ost-Süd-Ost",
            Self::SouthEast => "Süd-Ost",
            Self::SouthSouthEast => "Süd-Süd-Ost",
            Self::South => "Süd",
            Self::SouthSouthWest => "Süd-Süd-West",
            Self::SouthWest => "Süd-West",
            Self::WestSouthWest => "West-Süd-West",
            Self::West => "West",
            Self::WestNorthWest => "West-Nord-West",
            Self::NorthWest => "Nord-West",
            Self::NorthNorthWest => "Nord-Nord-West",
        }
    }
}

impl WindDirection {
    /// Returns the compass abbreviation (e.g. "N", "NNE", "NE").
    pub fn abbreviation(&self) -> &'static str {
        match self {
            Self::North => "N",
            Self::NorthNorthEast => "NNE",
            Self::NorthEast => "NE",
            Self::EastNorthEast => "ENE",
            Self::East => "E",
            Self::EastSouthEast => "ESE",
            Self::SouthEast => "SE",
            Self::SouthSouthEast => "SSE",
            Self::South => "S",
            Self::SouthSouthWest => "SSW",
            Self::SouthWest => "SW",
            Self::WestSouthWest => "WSW",
            Self::West => "W",
            Self::WestNorthWest => "WNW",
            Self::NorthWest => "NW",
            Self::NorthNorthWest => "NNW",
        }
    }
}

impl std::fmt::Display for WindDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for WindDirection {
    fn get_icon_color(&self) -> Option<Color> {
        None
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                Self::North | Self::NorthNorthEast => "nf-weather-wind_north",
                Self::NorthEast | Self::EastNorthEast => "nf-weather-wind_north_east",
                Self::East | Self::EastSouthEast => "nf-weather-wind_east",
                Self::SouthEast | Self::SouthSouthEast => "nf-weather-wind_south_east",
                Self::South | Self::SouthSouthWest => "nf-weather-wind_south",
                Self::SouthWest | Self::WestSouthWest => "nf-weather-wind_south_west",
                Self::West | Self::WestNorthWest => "nf-weather-wind_west",
                Self::NorthWest | Self::NorthNorthWest => "nf-weather-wind_north_west",
            }
            .to_string(),
        )
    }
}
