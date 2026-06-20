use crate::SwipeLauncherArguments;
use crate::config::area::config_entry::ConfigEntry;
use crate::config::error::ConfigValidationError;
use crate::config::layer::LayerConfigFile;
use crate::config::layout::config::LayoutConfig;
use crate::config::layout::profile::LayoutProfile;
use crate::config::layout::trigger::LayoutTrigger;
use crate::config::merge::MergeWithArguments;
use crate::config::rotation::RotationConfigFile;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use smearor_model_area::AreaConfig;
use smearor_model_area::AreaType;
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

    /// Global default configurations for plugin types.
    /// Each key names a default template (e.g. "button", "close_button")
    /// that individual plugin configs can reference via `defaults = "name"`.
    #[serde(default)]
    pub defaults: HashMap<String, Value>,

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

    /// Resolve global defaults for plugin configs.
    ///
    /// Each plugin config can reference a default template via
    /// `defaults = "template_name"`. The template values are merged
    /// with the instance-specific values, where instance values win.
    /// The `defaults` key itself is stripped from the final config.
    pub fn resolve_defaults(&mut self) {
        if self.defaults.is_empty() {
            return;
        }

        // Collect plugin IDs and their referenced default templates
        let plugins_to_merge: Vec<(String, String)> = self
            .entries
            .iter()
            .filter_map(|(id, entry)| {
                if let ConfigEntry::Plugin(config) = entry {
                    config
                        .get("defaults")
                        .and_then(|v| v.as_str())
                        .map(|template| (id.clone(), template.to_string()))
                } else {
                    None
                }
            })
            .collect();

        for (plugin_id, template_name) in plugins_to_merge {
            if let Some(default_config) = self.defaults.get(&template_name) {
                if let Some(ConfigEntry::Plugin(instance_config)) = self.entries.get(&plugin_id) {
                    let mut merged = default_config.clone();

                    if let Some(merged_obj) = merged.as_object_mut() {
                        if let Some(instance_obj) = instance_config.as_object() {
                            for (key, value) in instance_obj {
                                if key == "defaults" {
                                    continue;
                                }
                                merged_obj.insert(key.clone(), value.clone());
                            }
                        }
                    }

                    self.entries.insert(plugin_id, ConfigEntry::Plugin(merged));
                }
            }
        }
    }

    /// Resolve `include` directives in area configurations by loading
    /// external TOML files and merging them with the main config.
    ///
    /// Include files are TOML files where the root table contains area
    /// configuration fields and plugin configuration sections.
    pub fn resolve_includes(&mut self, base_path: &std::path::Path) -> Result<(), ConfigValidationError> {
        let base_dir = base_path.parent().unwrap_or_else(|| std::path::Path::new("."));

        // Collect (area_id, include_path) pairs to avoid borrow issues
        let mut includes_to_resolve: Vec<(String, String)> = Vec::new();
        for (area_id, entry) in &self.entries {
            if let ConfigEntry::Area(area) = entry {
                if let Some(include_path) = &area.include {
                    includes_to_resolve.push((area_id.clone(), include_path.clone()));
                }
            }
        }

        const AREA_CONFIG_KEYS: &[&str] = &[
            "area_type",
            "width",
            "width_percent",
            "min_width",
            "max_width",
            "open_transition",
            "close_transition",
            "auto_close",
            "close_on_escape",
            "align",
            "css_classes",
            "spacing",
            "plugins",
            "include",
        ];

        for (area_id, include_path) in includes_to_resolve {
            let full_path = base_dir.join(&include_path);
            let include_content = std::fs::read_to_string(&full_path).map_err(|e| ConfigValidationError::IncludeNotFound {
                path: include_path.clone(),
                area_id: area_id.clone(),
                reason: e.to_string(),
            })?;

            // Parse included file as generic TOML
            let include_toml: toml::Value = toml::from_str(&include_content).map_err(|e| ConfigValidationError::InvalidInclude {
                path: include_path.clone(),
                area_id: area_id.clone(),
                reason: e.to_string(),
            })?;

            // Convert to JSON for easier manipulation
            let include_json: Value = serde_json::to_value(&include_toml).map_err(|e| ConfigValidationError::InvalidInclude {
                path: include_path.clone(),
                area_id: area_id.clone(),
                reason: e.to_string(),
            })?;

            // Get current area config as JSON
            let main_area_json = if let Some(ConfigEntry::Area(area)) = self.entries.get(&area_id) {
                serde_json::to_value(area).map_err(|e| ConfigValidationError::InvalidInclude {
                    path: include_path.clone(),
                    area_id: area_id.clone(),
                    reason: e.to_string(),
                })?
            } else {
                continue;
            };

            // Start with include as base, remove its own include to prevent loops
            let mut merged_area_json = include_json.clone();
            if let Some(obj) = merged_area_json.as_object_mut() {
                obj.remove("include");
            }

            // Apply main config overrides
            if let Some(main_obj) = main_area_json.as_object() {
                for (key, value) in main_obj {
                    if key == "plugins" {
                        // Main plugins come first, then include plugins
                        if let Some(main_plugins) = value.as_array() {
                            let include_plugins = merged_area_json.get("plugins").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                            let mut combined = main_plugins.clone();
                            combined.extend(include_plugins);
                            merged_area_json["plugins"] = Value::Array(combined);
                        }
                    } else if key == "include" {
                        // Remove to prevent infinite loops
                        if let Some(obj) = merged_area_json.as_object_mut() {
                            obj.remove("include");
                        }
                    } else if AREA_CONFIG_KEYS.contains(&key.as_str()) {
                        // Main config overrides include for known area fields
                        merged_area_json[key] = value.clone();
                    }
                    // Unknown keys in main area config are ignored (handled as plugin configs)
                }
            }

            // Parse merged JSON back to AreaConfig
            let merged_area: AreaConfig = serde_json::from_value(merged_area_json).map_err(|e| ConfigValidationError::InvalidInclude {
                path: include_path.clone(),
                area_id: area_id.clone(),
                reason: e.to_string(),
            })?;

            // Update the area config
            self.entries.insert(area_id.clone(), ConfigEntry::Area(merged_area));

            // Extract plugin configs from include file
            if let Some(include_obj) = include_json.as_object() {
                for (plugin_id, plugin_value) in include_obj {
                    if AREA_CONFIG_KEYS.contains(&plugin_id.as_str()) {
                        continue;
                    }

                    // Only insert if not already present in main config
                    if !self.entries.contains_key(plugin_id) {
                        self.entries.insert(plugin_id.clone(), ConfigEntry::Plugin(plugin_value.clone()));
                    }
                }
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
