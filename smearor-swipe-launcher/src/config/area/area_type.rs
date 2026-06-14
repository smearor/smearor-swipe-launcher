use serde::Deserialize;

/// Defines the type of area in the layout
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub enum AreaType {
    /// Fixed width area that does not scroll
    #[serde(alias = "fixed", alias = "FIXED", alias = "Fixed")]
    #[default]
    Fixed,
    /// Scrollable area that can contain overflowing content
    #[serde(alias = "scroll", alias = "SCROLL", alias = "Scroll")]
    Scroll,
}
