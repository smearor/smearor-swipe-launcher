use crate::config::area::area_type::AreaType;
use crate::config::area::transition::AreaTransition;
use crate::config::plugin::PluginEntry;
use serde::Deserialize;

pub const DEFAULT_AREA_WIDTH: i32 = 200;

/// Configuration for a single area in the layout
#[derive(Debug, Clone, Deserialize)]
pub struct AreaConfig {
    /// Type of the area (fixed or scrollable)
    #[serde(default)]
    pub area_type: AreaType,

    /// Fixed width in pixels (if specified)
    #[serde(default = "default_width")]
    pub width: Option<i32>,

    /// Width as percentage of available space (alternative to width)
    #[serde(default)]
    pub width_percent: Option<f32>,

    /// Minimum width constraint in pixels
    #[serde(default)]
    pub min_width: Option<i32>,

    /// Maximum width constraint in pixels
    #[serde(default)]
    pub max_width: Option<i32>,

    /// Transition animation for area appearance/disappearance
    #[serde(default)]
    pub transition: AreaTransition,

    /// Automatically close the area when interaction ends
    #[serde(default)]
    pub auto_close: bool,

    /// Close the area when escape key is pressed
    #[serde(default)]
    pub close_on_escape: bool,

    /// List of plugins to load in this area
    pub plugins: Vec<PluginEntry>,
}

fn default_width() -> Option<i32> {
    Some(DEFAULT_AREA_WIDTH)
}

impl Default for AreaConfig {
    fn default() -> Self {
        AreaConfig {
            area_type: Default::default(),
            width: default_width(),
            width_percent: None,
            min_width: None,
            max_width: None,
            transition: Default::default(),
            auto_close: false,
            close_on_escape: false,
            plugins: Vec::new(),
        }
    }
}

impl AreaConfig {
    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.id.clone()).collect()
    }
}
