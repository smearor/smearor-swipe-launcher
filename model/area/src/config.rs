use crate::AreaTransition;
use crate::AreaTransitionStabby;
use crate::AreaType;
use crate::AreaTypeStabby;
use serde::Deserialize;
use serde::Serialize;
use smearor_model_plugin::PluginEntry;
use smearor_model_plugin::PluginEntryStabby;

pub const DEFAULT_AREA_WIDTH: i32 = 100;

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

    /// Path to an included area configuration file (relative to the main config)
    #[serde(default)]
    pub include: Option<String>,

    /// Additional CSS classes for styling the area container
    #[serde(default)]
    pub css_classes: Vec<String>,

    /// List of plugins to load in this area
    pub plugins: Vec<PluginEntry>,
}

/// ABI-stable version of `AreaConfig` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct AreaConfigStabby {
    pub area_type: AreaTypeStabby,
    pub width: stabby::option::Option<i32>,
    pub width_percent: stabby::option::Option<f32>,
    pub min_width: stabby::option::Option<i32>,
    pub max_width: stabby::option::Option<i32>,
    pub open_transition: AreaTransitionStabby,
    pub close_transition: stabby::option::Option<AreaTransitionStabby>,
    pub auto_close: bool,
    pub close_on_escape: bool,
    pub plugins: stabby::vec::Vec<PluginEntryStabby>,
}

impl From<AreaConfig> for AreaConfigStabby {
    fn from(value: AreaConfig) -> Self {
        Self {
            area_type: value.area_type.into(),
            width: value.width.map(Into::into).into(),
            width_percent: value.width_percent.map(Into::into).into(),
            min_width: value.min_width.map(Into::into).into(),
            max_width: value.max_width.map(Into::into).into(),
            open_transition: value.open_transition.into(),
            close_transition: value.close_transition.map(Into::into).into(),
            auto_close: value.auto_close,
            close_on_escape: value.close_on_escape,
            plugins: value.plugins.into_iter().map(Into::into).collect(),
            // css_classes intentionally omitted from stabby (not needed over FFI)
        }
    }
}

impl From<AreaConfigStabby> for AreaConfig {
    fn from(value: AreaConfigStabby) -> Self {
        Self {
            area_type: value.area_type.into(),
            width: {
                let opt: Option<i32> = value.width.into();
                opt
            },
            width_percent: {
                let opt: Option<f32> = value.width_percent.into();
                opt
            },
            min_width: {
                let opt: Option<i32> = value.min_width.into();
                opt
            },
            max_width: {
                let opt: Option<i32> = value.max_width.into();
                opt
            },
            open_transition: value.open_transition.into(),
            close_transition: {
                let opt: Option<AreaTransitionStabby> = value.close_transition.into();
                opt.map(Into::into)
            },
            auto_close: value.auto_close,
            close_on_escape: value.close_on_escape,
            include: None,
            css_classes: Vec::new(),
            plugins: value.plugins.into_iter().map(Into::into).collect(),
        }
    }
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
            include: None,
            css_classes: Vec::new(),
            plugins: Vec::new(),
        }
    }
}

fn default_width() -> Option<i32> {
    Some(DEFAULT_AREA_WIDTH)
}
