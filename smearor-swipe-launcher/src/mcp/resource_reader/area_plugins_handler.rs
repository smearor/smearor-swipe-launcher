use super::common::with_first_area_manager;
use super::registry::McpResourceHandler;
use crate::config::area::config_entry::ConfigEntry;
use crate::host::LauncherHost;
use typed_builder::TypedBuilder;

/// Summary of a plugin's identity fields for the `area://plugins` resource.
#[derive(Debug, Clone, serde::Serialize, TypedBuilder)]
struct PluginInfo {
    /// The plugin ID.
    id: String,
    /// The path to the shared library, if specified.
    path: Option<String>,
    /// The short name used for library resolution, if specified.
    name: Option<String>,
    /// The widget type to instantiate, if the plugin provides multiple widgets.
    widget: Option<String>,
}

/// A group of plugins belonging to a single area.
#[derive(Debug, Clone, serde::Serialize, TypedBuilder)]
struct AreaPlugins {
    /// The area ID.
    area_id: String,
    /// All enabled plugins in this area.
    plugins: Vec<PluginInfo>,
}

/// Handler for `area://plugins` — returns all plugins grouped by area.
pub struct AreaPluginsHandler;

impl McpResourceHandler for AreaPluginsHandler {
    fn uri_matches(&self, uri: &str) -> bool {
        uri == "area://plugins"
    }

    fn handle(&self, host: &LauncherHost, _uri: &str) -> Result<String, String> {
        with_first_area_manager(host, |area_manager| {
            let config = area_manager.config();
            let areas: Vec<AreaPlugins> = config
                .entries
                .iter()
                .filter_map(|(area_id, entry)| {
                    let area_config = match entry {
                        ConfigEntry::Area(ac) => ac,
                        ConfigEntry::Plugin(_) => return None,
                    };
                    let plugins: Vec<PluginInfo> = area_config
                        .plugins
                        .iter()
                        .filter(|p| !p.disabled)
                        .map(|p| {
                            PluginInfo::builder()
                                .id(p.id.clone())
                                .path(p.path.clone())
                                .name(p.name.clone())
                                .widget(p.widget.clone())
                                .build()
                        })
                        .collect();
                    Some(AreaPlugins::builder().area_id(area_id.clone()).plugins(plugins).build())
                })
                .collect();
            serde_json::to_string(&serde_json::json!({ "areas": areas })).map_err(|e| e.to_string())
        })
    }
}
