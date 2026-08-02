use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::Color;
use smearor_swipe_launcher_plugin_api::WidgetIconRendering;

/// UV index exposure category with sun-protection advice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UvIndexLevel {
    /// 0–3 — low exposure, no protection needed.
    Low,
    /// 3–6 — moderate exposure, sun protection recommended.
    Moderate,
    /// 6–8 — high exposure, seek shade and use sunscreen.
    High,
    /// 8–11 — very high exposure, avoid direct sun.
    VeryHigh,
    /// Above 11 — extreme radiation, avoid outdoor exposure.
    Extreme,
}

impl From<f32> for UvIndexLevel {
    fn from(uv: f32) -> Self {
        match uv {
            u if u < 3.0 => Self::Low,
            u if u < 6.0 => Self::Moderate,
            u if u < 8.0 => Self::High,
            u if u < 11.0 => Self::VeryHigh,
            _ => Self::Extreme,
        }
    }
}

impl AsRef<str> for UvIndexLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Low => "niedrige UV-Belastung, kein Schutz erforderlich",
            Self::Moderate => "mittlere UV-Belastung, Sonnenschutz empfohlen",
            Self::High => "hohe UV-Belastung, Schatten suchen und Sonnencreme nutzen",
            Self::VeryHigh => "sehr hohe UV-Belastung, direkte Sonne meiden",
            Self::Extreme => "extreme UV-Strahlung, Aufenthalt im Freien vermeiden",
        }
    }
}

impl std::fmt::Display for UvIndexLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl WidgetIconRendering for UvIndexLevel {
    fn get_icon_color(&self) -> Option<Color> {
        let color = match self {
            Self::Low => Color::GREEN,
            Self::Moderate => Color::YELLOW,
            Self::High => Color::ORANGE,
            Self::VeryHigh => Color::RED,
            Self::Extreme => Color::DARK_RED,
        };
        Some(color)
    }

    fn get_icon_name(&self) -> Option<String> {
        Some(
            match self {
                Self::Low => "nf-fa-sun_o",
                Self::Moderate => "nf-fa-glasses",
                Self::High => "nf-fa-umbrella_beach",
                Self::VeryHigh => "nf-fa-exclamation_triangle",
                Self::Extreme => "nf-fa-radiation",
            }
            .to_string(),
        )
    }
}
