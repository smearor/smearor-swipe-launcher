use abi_stable::StableAbi;
use abi_stable::derive_macro_reexports::ROption;
use abi_stable::std_types::RString;
use serde::Deserialize;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

pub static PLUGIN_ID: AtomicU32 = AtomicU32::new(0);

#[repr(C)]
#[derive(Debug, Clone, Deserialize, StableAbi)]
pub struct PluginMeta {
    pub id: RString,
    pub display_name: RString,
    pub icon_name: ROption<RString>,
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
