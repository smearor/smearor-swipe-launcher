use abi_stable::StableAbi;
use abi_stable::derive_macro_reexports::ROption;
use abi_stable::std_types::RString;
use serde::Deserialize;

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
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
    "plugin".to_string()
}

fn default_plugin_display_name() -> String {
    "Plugin".to_string()
}
