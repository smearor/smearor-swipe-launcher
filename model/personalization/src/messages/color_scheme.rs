use serde::Deserialize;
use serde::Serialize;

/// The user's preferred color scheme.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorScheme {
    /// Follow system settings (default).
    #[default]
    System,
    /// Light mode.
    Light,
    /// Dark mode.
    Dark,
}

impl std::str::FromStr for ColorScheme {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "System" => Ok(ColorScheme::System),
            "Light" => Ok(ColorScheme::Light),
            "Dark" => Ok(ColorScheme::Dark),
            _ => Err(format!("Unknown color scheme: {s}")),
        }
    }
}
