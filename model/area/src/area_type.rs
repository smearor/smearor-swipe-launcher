use serde::Deserialize;
use serde::Serialize;

/// Defines the type of area in the layout
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum AreaType {
    /// Fixed width area that does not scroll
    #[serde(alias = "fixed", alias = "FIXED", alias = "Fixed")]
    #[default]
    Fixed,

    /// Scrollable area that can contain overflowing content
    #[serde(alias = "scroll", alias = "SCROLL", alias = "Scroll")]
    Scroll,
}

/// ABI-stable version of `AreaType` for cross-plugin messaging.
#[repr(u8)]
#[stabby::stabby]
#[derive(Debug, Clone, Default, PartialEq)]
pub enum AreaTypeStabby {
    #[default]
    Fixed,
    Scroll,
}

impl From<AreaType> for AreaTypeStabby {
    fn from(value: AreaType) -> Self {
        match value {
            AreaType::Fixed => AreaTypeStabby::Fixed,
            AreaType::Scroll => AreaTypeStabby::Scroll,
        }
    }
}

impl From<AreaTypeStabby> for AreaType {
    fn from(value: AreaTypeStabby) -> Self {
        match value {
            AreaTypeStabby::Fixed => AreaType::Fixed,
            AreaTypeStabby::Scroll => AreaType::Scroll,
        }
    }
}
