use serde::Deserialize;

/// Defines the orientation of the layout
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum Orientation {
    /// Horizontal layout (left to right)
    #[serde(alias = "horizontal", alias = "HORIZONTAL", alias = "Horizontal")]
    Horizontal,
    /// Vertical layout (top to bottom)
    #[serde(alias = "vertical", alias = "VERTICAL", alias = "Vertical")]
    Vertical,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::Horizontal
    }
}

impl From<&Orientation> for gtk4::Orientation {
    fn from(orientation: &Orientation) -> Self {
        match orientation {
            Orientation::Horizontal => gtk4::Orientation::Horizontal,
            Orientation::Vertical => gtk4::Orientation::Vertical,
        }
    }
}
