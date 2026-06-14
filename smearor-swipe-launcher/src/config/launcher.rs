use crate::SwipeLauncherArguments;
use crate::config::area::area_type::AreaType;
use crate::config::area::config::AreaConfig;
use crate::config::area::config_entry::ConfigEntry;
use crate::config::error::ConfigValidationError;
use crate::config::layer::LayerConfigFile;
use crate::config::layout::config::LayoutConfig;
use crate::config::layout::profile::LayoutProfile;
use crate::config::layout::trigger::LayoutTrigger;
use crate::config::merge::MergeWithArguments;
use crate::config::plugin::PluginEntry;
use crate::config::rotation::RotationConfigFile;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use std::collections::HashMap;
use tracing::warn;

/// Main configuration for the swipe launcher
#[derive(Debug, Clone, Deserialize)]
pub struct SwipeLauncherConfig {
    /// Default layout area order
    pub areas: Vec<String>,

    /// Launcher settings (layer, rotation, etc.)
    pub launcher: SwipeLauncherSettings,

    /// Layout configuration
    #[serde(default)]
    pub layout: LayoutConfig,

    /// Alternative layout profiles for different contexts
    #[serde(default)]
    pub profiles: Vec<LayoutProfile>,

    /// Services to load
    #[serde(default)]
    pub services: Vec<PluginEntry>,

    /// Area configurations and plugin configs keyed by ID
    #[serde(flatten)]
    pub entries: HashMap<String, ConfigEntry>,
}

impl SwipeLauncherConfig {
    /// Get area configuration by ID
    pub fn get_area_config(&self, area_id: &str) -> Option<&AreaConfig> {
        self.entries.get(area_id).and_then(|entry| match entry {
            ConfigEntry::Area(config) => Some(config),
            ConfigEntry::Plugin(_) => None,
        })
    }

    /// Get plugin configuration by ID
    pub fn get_plugin_config(&self, plugin_id: &str) -> Option<&Value> {
        self.entries.get(plugin_id).and_then(|entry| match entry {
            ConfigEntry::Area(_) => None,
            ConfigEntry::Plugin(value) => Some(value),
        })
    }

    /// Get plugin config for plugin API (legacy method for compatibility)
    pub fn plugin_config(&self, id: &str) -> PluginConfig {
        let config = self.get_plugin_config(id).cloned().unwrap_or_else(|| {
            warn!("No config found for plugin {id}, using empty config");
            json!({})
        });
        PluginConfig { config }
    }

    /// Get layout for specific context (monitor/workspace)
    pub fn get_layout_for_context(&self, monitor: Option<&str>, workspace: Option<i32>) -> (&Vec<String>, &HashMap<String, ConfigEntry>) {
        for profile in &self.profiles {
            match &profile.trigger {
                LayoutTrigger::MonitorWorkspace { monitor: m, workspace: w } => {
                    if Some(m.as_str()) == monitor && Some(*w) == workspace {
                        return (&profile.areas, &profile.entries);
                    }
                }
                LayoutTrigger::Monitor(m) => {
                    if Some(m.as_str()) == monitor {
                        return (&profile.areas, &profile.entries);
                    }
                }
                LayoutTrigger::Workspace(w) => {
                    if Some(*w) == workspace {
                        return (&profile.areas, &profile.entries);
                    }
                }
                LayoutTrigger::Default => {}
            }
        }
        (&self.areas, &self.entries)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.areas.is_empty() {
            return Err(ConfigValidationError::NoAreasDefined);
        }

        let mut seen_areas = std::collections::HashSet::new();
        for area_id in &self.areas {
            if !seen_areas.insert(area_id) {
                return Err(ConfigValidationError::DuplicateAreaId { area_id: area_id.clone() });
            }
        }

        for area_id in &self.areas {
            if self.get_area_config(area_id).is_none() {
                return Err(ConfigValidationError::AreaNotFound { area_id: area_id.clone() });
            }
        }

        for area_id in &self.areas {
            if let Some(area_config) = self.get_area_config(area_id) {
                self.validate_area_config(area_id, area_config)?;
            }
        }

        let mut seen_plugins = std::collections::HashSet::new();
        for area_id in &self.areas {
            if let Some(area_config) = self.get_area_config(area_id) {
                for plugin in &area_config.plugins {
                    if !seen_plugins.insert(&plugin.id) {
                        return Err(ConfigValidationError::DuplicatePluginId { plugin_id: plugin.id.clone() });
                    }
                }
            }
        }

        for profile in &self.profiles {
            self.validate_layout_profile(profile)?;
        }

        Ok(())
    }

    fn validate_area_config(&self, area_id: &str, area_config: &AreaConfig) -> Result<(), ConfigValidationError> {
        match area_config.area_type {
            AreaType::Fixed => {
                if area_config.width.is_none() && area_config.width_percent.is_none() {
                    return Err(ConfigValidationError::MissingWidthSpec { area_id: area_id.to_string() });
                }

                if let Some(percent) = area_config.width_percent {
                    if percent <= 0.0 || percent > 1.0 {
                        return Err(ConfigValidationError::InvalidWidthPercent {
                            area_id: area_id.to_string(),
                            percent,
                        });
                    }
                }
            }
            AreaType::Scroll => {}
        }

        Ok(())
    }

    fn validate_layout_profile(&self, profile: &LayoutProfile) -> Result<(), ConfigValidationError> {
        for area_id in &profile.areas {
            if !profile.entries.contains_key(area_id) {
                return Err(ConfigValidationError::AreaNotFound { area_id: area_id.clone() });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SwipeLauncherSettings {
    /// Configuration for the layer
    #[serde(default, flatten)]
    pub layer: LayerConfigFile,

    /// Configuration for the rotation
    #[serde(default, flatten)]
    pub rotation: RotationConfigFile,
}

impl MergeWithArguments<SwipeLauncherArguments> for SwipeLauncherSettings {
    fn merge_with_arguments(self, args: &SwipeLauncherArguments) -> Self {
        let mut config = self;
        config.rotation = config.rotation.merge_with_arguments(&args.rotation);
        config.layer = config.layer.merge_with_arguments(&args.layer);
        config
    }
}
