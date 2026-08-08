use super::common::with_first_area_manager;
use super::registry::McpResourceHandler;
use crate::config::area::config_entry::ConfigEntry;
use crate::host::LauncherHost;

/// Handler for `plugin://list` — returns all services and widget plugins.
pub struct PluginListHandler;

impl McpResourceHandler for PluginListHandler {
    fn uri_matches(&self, uri: &str) -> bool {
        uri == "plugin://list"
    }

    fn handle(&self, host: &LauncherHost, _uri: &str) -> Result<String, String> {
        let mut plugins: Vec<serde_json::Value> = Vec::new();

        if let Ok(guard) = host.services_config.lock() {
            if let Some(services_config) = guard.as_ref() {
                for service in &services_config.services {
                    plugins.push(serde_json::json!({
                        "id": service.id,
                        "path": service.path,
                        "name": service.name,
                        "type": "service",
                    }));
                }
            }
        }

        with_first_area_manager(host, |area_manager| {
            let config = area_manager.config();
            for (_area_id, entry) in &config.entries {
                let area_config = match entry {
                    ConfigEntry::Area(ac) => ac,
                    ConfigEntry::Plugin(_) => continue,
                };
                for plugin in &area_config.plugins {
                    if !plugin.disabled {
                        plugins.push(serde_json::json!({
                            "id": plugin.id,
                            "path": plugin.path,
                            "name": plugin.name,
                            "type": "widget",
                        }));
                    }
                }
            }
            Ok(())
        })?;

        serde_json::to_string(&serde_json::json!({ "plugins": plugins })).map_err(|e| e.to_string())
    }
}
