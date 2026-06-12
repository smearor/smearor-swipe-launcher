use crate::config::area::config_entry::ConfigEntry;
use crate::config::layout::trigger::LayoutTrigger;
use serde::Deserialize;
use std::collections::HashMap;

/// Defines a specific layout profile with trigger conditions and area configurations
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutProfile {
    /// Trigger condition for when this profile should be active
    pub trigger: LayoutTrigger,
    /// Ordered list of area IDs in this profile
    pub areas: Vec<String>,
    /// Area and plugin configurations keyed by area/plugin ID
    #[serde(flatten)]
    pub entries: HashMap<String, ConfigEntry>,
}
