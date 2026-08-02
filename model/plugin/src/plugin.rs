use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// The instance id of the plugin
    pub id: String,

    /// The path to the shared library of the plugin (.so file)
    ///
    /// Either `path` or `name` must be specified. When `path` is given, it is
    /// used directly (relative to the working directory or absolute). When
    /// `name` is given instead, the host resolves the library by searching
    /// standard directories for `libsmearor_<name>.so`.
    #[serde(default)]
    pub path: Option<String>,

    /// The short name of the plugin, used to resolve the shared library path.
    ///
    /// The host searches for `libsmearor_<name>.so` in the following
    /// directories (first match wins):
    /// - `~/.local/lib/smearor/` (user-local)
    /// - `/usr/local/lib/smearor/` (system-wide)
    ///
    /// Either `path` or `name` must be specified.
    #[serde(default)]
    pub name: Option<String>,

    /// The widget type to instantiate from a plugin that provides multiple widgets.
    ///
    /// This field is optional. When present, the host passes it to the plugin
    /// through the `widget` field in the plugin configuration.
    pub widget: Option<String>,

    /// Whether this plugin entry is disabled and should be skipped during loading.
    ///
    /// Defaults to `false`. When set to `true`, the host does not load the
    /// plugin library nor add it to any area.
    #[serde(default)]
    pub disabled: bool,

    /// Span group identifier for Multi-Span Widgets.
    ///
    /// Plugins with the same `span_group` form a single logical widget that
    /// spans multiple buttons. The host renders the group at combined dimensions
    /// and splits the result across the buttons.
    #[serde(default)]
    pub span_group: Option<String>,

    /// Index within the span group (0-based).
    ///
    /// Determines the order of buttons within a span group. The host sorts
    /// plugins by `span_index` before rendering.
    #[serde(default)]
    pub span_index: Option<u32>,

    /// Number of rows this span group occupies in the button grid.
    ///
    /// Defaults to `1` (horizontal span). Used together with `span_cols`
    /// to determine the combined render dimensions and physical button
    /// mapping for 2D span groups.
    #[serde(default)]
    pub span_rows: Option<u32>,

    /// Number of columns this span group occupies in the button grid.
    ///
    /// Defaults to `1` (vertical span or single button). Used together
    /// with `span_rows` to determine the combined render dimensions and
    /// physical button mapping for 2D span groups.
    #[serde(default)]
    pub span_cols: Option<u32>,
}

/// ABI-stable version of `PluginEntry` for cross-plugin messaging.
#[stabby::stabby(no_opt)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginEntryStabby {
    pub id: stabby::string::String,
    pub path: stabby::string::String,
    pub widget: stabby::option::Option<stabby::string::String>,
}

impl From<PluginEntry> for PluginEntryStabby {
    fn from(value: PluginEntry) -> Self {
        Self {
            id: value.id.into(),
            path: value.path.unwrap_or_default().into(),
            widget: value.widget.map(|widget| widget.into()).into(),
        }
    }
}

impl From<PluginEntryStabby> for PluginEntry {
    fn from(value: PluginEntryStabby) -> Self {
        let path = value.path.to_string();
        Self {
            id: value.id.to_string(),
            path: if path.is_empty() { None } else { Some(path) },
            name: None,
            widget: {
                let widget: Option<stabby::string::String> = value.widget.into();
                widget.map(|widget| widget.to_string())
            },
            disabled: false,
            span_group: None,
            span_index: None,
            span_rows: None,
            span_cols: None,
        }
    }
}
