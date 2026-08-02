use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// Total precipitation amount category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationAmountLevel {
    /// 0–0.1 mm — no significant precipitation.
    None,
    /// 0.1–5 mm — a few light drops.
    LightDrops,
    /// 5–20 mm — moderate rain.
    Moderate,
    /// Above 20 mm — heavy continuous rain.
    Heavy,
}

impl From<f32> for PrecipitationAmountLevel {
    fn from(mm: f32) -> Self {
        match mm {
            m if m <= 0.1 => Self::None,
            m if m < 5.0 => Self::LightDrops,
            m if m < 20.0 => Self::Moderate,
            _ => Self::Heavy,
        }
    }
}

impl AsRef<str> for PrecipitationAmountLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::None => "kein nennenswerter Niederschlag",
            Self::LightDrops => "ein paar leichte Tropfen",
            Self::Moderate => "mäßig viel Regen",
            Self::Heavy => "ergiebiger Dauerregen",
        }
    }
}

impl std::fmt::Display for PrecipitationAmountLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for PrecipitationAmountLevel {
    fn get_icon_color(&self) -> Option<Color> {
        Some(match self {
            Self::None => Color::GREEN,
            Self::LightDrops => Color::LIGHT_GREEN,
            Self::Moderate => Color::ORANGE,
            Self::Heavy => Color::RED,
        })
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                Self::None => "nf-fa-hotjar",
                Self::LightDrops => "nf-weather-sprinkle",
                Self::Moderate => "nf-weather-rain",
                Self::Heavy => "nf-weather-showers",
            }
            .to_string(),
        )
    }
}
