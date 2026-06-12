use crate::config::area::orientation::Orientation;
use serde::Deserialize;

/// Configuration for the overall layout structure
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutConfig {
    /// Orientation of the layout (horizontal or vertical)
    #[serde(default)]
    pub orientation: Orientation,

    /// Spacing between areas in pixels
    #[serde(default)]
    pub spacing: i32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        LayoutConfig {
            orientation: Orientation::Horizontal,
            spacing: 0,
        }
    }
}
