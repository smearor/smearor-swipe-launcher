use crate::PluginConstructionError;
use crate::PluginMeta;
use crate::PluginMetaGetter;
use crate::PluginMetaRaw;
use serde::Deserialize;
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    pub config: Value,
}

impl PluginConfig {
    pub fn new(config_json: *const i8, config_len: usize) -> Result<Self, PluginConstructionError> {
        if config_json.is_null() {
            return Err(PluginConstructionError::ConfigJsonIsNull);
        }
        let slice = unsafe { std::slice::from_raw_parts(config_json as *const u8, config_len) };
        Self::from_str(std::str::from_utf8(slice).map_err(|e| PluginConstructionError::InvalidUtf8Config(e.to_string().into()))?)
            .map_err(|e| PluginConstructionError::FailedToParseConfig(e.to_string().into()))
    }
}

impl PluginMetaGetter for PluginConfig {
    fn meta(&self) -> PluginMeta {
        PluginMeta::try_from(self).expect("failed to convert PluginConfig to PluginMeta")
    }
}

impl FromStr for PluginConfig {
    type Err = serde_json::Error;

    fn from_str(config_str: &str) -> Result<Self, Self::Err> {
        Ok(PluginConfig {
            config: serde_json::from_str(config_str)?,
        })
    }
}
impl From<Value> for PluginConfig {
    fn from(config: Value) -> Self {
        PluginConfig { config }
    }
}

impl TryFrom<&PluginConfig> for PluginMetaRaw {
    type Error = PluginConstructionError;

    fn try_from(config: &PluginConfig) -> Result<Self, Self::Error> {
        Ok(serde_json::from_value(config.config.clone()).map_err(|e| PluginConstructionError::FailedToParseMetaData(e.to_string().into()))?)
    }
}

impl TryFrom<&PluginConfig> for PluginMeta {
    type Error = PluginConstructionError;

    fn try_from(config: &PluginConfig) -> Result<Self, Self::Error> {
        let meta_raw = PluginMetaRaw::try_from(config)?;
        Ok(PluginMeta::new(meta_raw.id, meta_raw.display_name, meta_raw.icon_name))
    }
}
