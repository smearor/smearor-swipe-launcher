use serde::Deserialize;
use serde::Serialize;

/// PM2.5 particulate matter concentration category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticulateMatterLevel {
    /// 0–10 µg/m³ — very clean air.
    VeryClean,
    /// 10–25 µg/m³ — normal particulate values.
    Normal,
    /// 25–50 µg/m³ — elevated particulate concentration.
    Elevated,
    /// Above 50 µg/m³ — high particulate pollution.
    High,
}

impl From<f32> for ParticulateMatterLevel {
    fn from(micrograms: f32) -> Self {
        match micrograms {
            val if val < 10.0 => Self::VeryClean,
            val if val < 25.0 => Self::Normal,
            val if val < 50.0 => Self::Elevated,
            _ => Self::High,
        }
    }
}

impl AsRef<str> for ParticulateMatterLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::VeryClean => "sehr reine Luft",
            Self::Normal => "normale Feinstaubwerte",
            Self::Elevated => "erhöhte Feinstaubkonzentration",
            Self::High => "hohe Feinstaubbelastung",
        }
    }
}

impl std::fmt::Display for ParticulateMatterLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
