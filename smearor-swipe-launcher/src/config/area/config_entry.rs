use crate::config::area::config::AreaConfig;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;

/// Represents a configuration entry that can be either an area or a plugin configuration
#[derive(Debug, Clone)]
pub enum ConfigEntry {
    /// Area configuration
    Area(AreaConfig),
    /// Plugin configuration (raw JSON value)
    Plugin(Value),
}

impl<'de> Deserialize<'de> for ConfigEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        if let Some(obj) = value.as_object() {
            if obj.contains_key("area_type") || obj.contains_key("plugins") {
                if let Ok(area_config) = serde_json::from_value::<AreaConfig>(value.clone()) {
                    return Ok(ConfigEntry::Area(area_config));
                }
            }
        }

        Ok(ConfigEntry::Plugin(value))
    }
}
