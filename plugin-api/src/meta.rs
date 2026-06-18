use serde::Deserialize;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

pub static PLUGIN_ID: AtomicU32 = AtomicU32::new(0);

/// Metadata describing a plugin or service.
#[stabby::stabby]
#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub id: stabby::string::String,
    pub display_name: stabby::string::String,
    pub icon_name: stabby::option::Option<stabby::string::String>,
}

impl PluginMeta {
    pub fn new(id: String, display_name: String, icon_name: Option<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            icon_name: icon_name.map(|s| s.into()).into(),
        }
    }
}

/// Raw metadata from the config file, before conversion to PluginMeta.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct PluginMetaRaw {
    #[serde(default = "default_plugin_id")]
    pub id: String,
    #[serde(default = "default_plugin_display_name")]
    pub display_name: String,
    #[serde(default)]
    pub icon_name: Option<String>,
}

fn default_plugin_id() -> String {
    format!("plugin-{}", PLUGIN_ID.fetch_add(1, Ordering::SeqCst))
}

fn default_plugin_display_name() -> String {
    format!("Plugin {}", PLUGIN_ID.load(Ordering::SeqCst))
}

/// Trait for types that expose their plugin metadata.
pub trait PluginMetaGetter {
    fn meta(&self) -> PluginMeta;

    fn meta_raw(&self) -> PluginMetaRaw {
        let meta = self.meta();
        PluginMetaRaw {
            id: meta.id.to_string(),
            display_name: meta.display_name.to_string(),
            icon_name: {
                let opt: Option<stabby::string::String> = meta.icon_name.into();
                opt.map(|s| s.to_string())
            },
        }
    }
}
