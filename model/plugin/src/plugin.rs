use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// The instance id of the plugin
    pub id: String,

    /// The path to the shared library of the plugin (.so file)
    pub path: String,

    /// The widget type to instantiate from a plugin that provides multiple widgets.
    ///
    /// This field is optional. When present, the host passes it to the plugin
    /// through the `widget` field in the plugin configuration.
    pub widget: Option<String>,
}

/// ABI-stable version of `PluginEntry` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone)]
pub struct PluginEntryStabby {
    pub id: stabby::string::String,
    pub path: stabby::string::String,
    pub widget: stabby::option::Option<stabby::string::String>,
}

impl From<PluginEntry> for PluginEntryStabby {
    fn from(value: PluginEntry) -> Self {
        Self {
            id: value.id.into(),
            path: value.path.into(),
            widget: value.widget.map(|widget| widget.into()).into(),
        }
    }
}

impl From<PluginEntryStabby> for PluginEntry {
    fn from(value: PluginEntryStabby) -> Self {
        Self {
            id: value.id.to_string(),
            path: value.path.to_string(),
            widget: {
                let widget: Option<stabby::string::String> = value.widget.into();
                widget.map(|widget| widget.to_string())
            },
        }
    }
}
