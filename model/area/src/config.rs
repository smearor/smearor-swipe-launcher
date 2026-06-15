use crate::AreaTransition;
use crate::AreaType;
use serde::Deserialize;
use serde::Serialize;
use smearor_model_plugin::PluginEntry;

pub const DEFAULT_AREA_WIDTH: i32 = 200;

/// Configuration for a single area in the layout
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Transition animation for area appearance
    #[serde(default)]
    pub open_transition: AreaTransition,

    /// Transition animation for area disappearance (closes with same animation if not set)
    #[serde(default)]
    pub close_transition: Option<AreaTransition>,

    /// Automatically close the area when interaction ends
    #[serde(default)]
    pub auto_close: bool,

    /// Close the area when escape key is pressed
    #[serde(default)]
    pub close_on_escape: bool,

    /// List of plugins to load in this area
    pub plugins: Vec<PluginEntry>,
}

impl AreaConfig {
    pub fn open_transition(&self) -> AreaTransition {
        self.open_transition.clone()
    }
    pub fn close_transition(&self) -> AreaTransition {
        self.close_transition.clone().unwrap_or(self.open_transition.opposite())
    }

    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.id.clone()).collect()
    }
}

impl Default for AreaConfig {
    fn default() -> Self {
        AreaConfig {
            area_type: Default::default(),
            width: default_width(),
            width_percent: None,
            min_width: None,
            max_width: None,
            open_transition: Default::default(),
            close_transition: None,
            auto_close: false,
            close_on_escape: false,
            plugins: Vec::new(),
        }
    }
}

fn default_width() -> Option<i32> {
    Some(DEFAULT_AREA_WIDTH)
}
