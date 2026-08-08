use super::common::with_first_area_manager;
use super::registry::McpResourceHandler;
use crate::config::area::config_entry::ConfigEntry;
use crate::host::LauncherHost;
use typed_builder::TypedBuilder;

/// A button widget plugin entry with its area assignment and configuration.
#[derive(Debug, Clone, serde::Serialize, TypedBuilder)]
struct ButtonEntry {
    /// The plugin ID.
    id: String,
    /// The area ID this button belongs to.
    area_id: String,
    /// The plugin-specific configuration.
    config: serde_json::Value,
}

/// Handler for `area://buttons` — returns all button widget plugin configs.
pub struct AreaButtonsHandler;

impl McpResourceHandler for AreaButtonsHandler {
    fn uri_matches(&self, uri: &str) -> bool {
        uri == "area://buttons"
    }

    fn handle(&self, host: &LauncherHost, _uri: &str) -> Result<String, String> {
        with_first_area_manager(host, |area_manager| {
            let config = area_manager.config();
            let mut buttons: Vec<ButtonEntry> = Vec::new();
            for (area_id, entry) in &config.entries {
                let area_config = match entry {
                    ConfigEntry::Area(ac) => ac,
                    ConfigEntry::Plugin(_) => continue,
                };
                for plugin in &area_config.plugins {
                    if plugin.path.as_deref().unwrap_or("").contains("libsmearor_button_widget") && !plugin.disabled {
                        if let Some(button_config) = config.get_plugin_config(&plugin.id) {
                            buttons.push(
                                ButtonEntry::builder()
                                    .id(plugin.id.clone())
                                    .area_id(area_id.clone())
                                    .config(button_config.clone())
                                    .build(),
                            );
                        }
                    }
                }
            }
            serde_json::to_string(&serde_json::json!({ "buttons": buttons })).map_err(|e| e.to_string())
        })
    }
}
