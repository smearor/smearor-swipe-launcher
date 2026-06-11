use crate::PluginConstructionError;
use crate::PluginMeta;
use crate::PluginMetaGetter;
use crate::PluginMetaRaw;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    pub config: Value,
}

impl PluginMetaGetter for PluginConfig {
    fn meta(&self) -> PluginMeta {
        todo!()
    }
}

pub trait ParsePluginConfig {
    fn parse(config: &Value) -> Result<Self, PluginConstructionError>
    where
        Self: Sized;
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
