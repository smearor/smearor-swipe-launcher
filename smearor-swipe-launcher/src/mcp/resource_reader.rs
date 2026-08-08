use crate::area::instance_area_manager::InstanceAreaManager;
use crate::config::area::config_entry::ConfigEntry;
use crate::host::LauncherHost;

pub fn read_mcp_resource(host: &LauncherHost, uri: String) -> Result<String, String> {
    if uri == "area://list" {
        with_first_area_manager(host, |area_manager| {
            let areas = area_manager.list_areas();
            serde_json::to_string(&areas).map_err(|e| e.to_string())
        })
    } else if uri.starts_with("area://") && uri.ends_with("/state") {
        let area_id = uri.trim_start_matches("area://").trim_end_matches("/state");
        with_first_area_manager(host, |area_manager| {
            let areas = area_manager.list_areas();
            let area = areas.into_iter().find(|a| a.area_id == area_id).ok_or(format!("Area {} not found", area_id))?;
            serde_json::to_string(&area).map_err(|e| e.to_string())
        })
    } else if uri == "area://plugins" {
        with_first_area_manager(host, |area_manager| {
            let config = area_manager.config();
            let areas: Vec<serde_json::Value> = config
                .entries
                .iter()
                .filter_map(|(area_id, entry)| {
                    let area_config = match entry {
                        ConfigEntry::Area(ac) => ac,
                        ConfigEntry::Plugin(_) => return None,
                    };
                    let plugins: Vec<serde_json::Value> = area_config
                        .plugins
                        .iter()
                        .filter(|p| !p.disabled)
                        .map(|p| {
                            serde_json::json!({
                                "id": p.id,
                                "path": p.path,
                                "name": p.name,
                                "widget": p.widget,
                            })
                        })
                        .collect();
                    Some(serde_json::json!({
                        "area_id": area_id,
                        "plugins": plugins,
                    }))
                })
                .collect();
            serde_json::to_string(&serde_json::json!({ "areas": areas })).map_err(|e| e.to_string())
        })
    } else if uri == "area://buttons" {
        with_first_area_manager(host, |area_manager| {
            let config = area_manager.config();
            let mut buttons: Vec<serde_json::Value> = Vec::new();
            for (area_id, entry) in &config.entries {
                let area_config = match entry {
                    ConfigEntry::Area(ac) => ac,
                    ConfigEntry::Plugin(_) => continue,
                };
                for plugin in &area_config.plugins {
                    if plugin.path.as_deref().unwrap_or("").contains("libsmearor_button_widget") && !plugin.disabled {
                        if let Some(button_config) = config.get_plugin_config(&plugin.id) {
                            buttons.push(serde_json::json!({
                                "id": plugin.id,
                                "area_id": area_id,
                                "config": button_config,
                            }));
                        }
                    }
                }
            }
            serde_json::to_string(&serde_json::json!({ "buttons": buttons })).map_err(|e| e.to_string())
        })
    } else if uri == "plugin://list" {
        read_plugin_list(host)
    } else {
        Err(format!("Resource {} not implemented", uri))
    }
}

pub fn read_plugin_list(host: &LauncherHost) -> Result<String, String> {
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

fn with_first_area_manager<F, T>(host: &LauncherHost, callback: F) -> Result<T, String>
where
    F: FnOnce(&InstanceAreaManager) -> Result<T, String>,
{
    let instances = host.instances.lock().map_err(|_| "Failed to lock instances")?;
    let first_instance = instances.values().next().ok_or("No launcher instance available")?;
    let area_manager = first_instance.area_manager.lock().map_err(|_| "Failed to lock area manager")?;
    callback(&area_manager)
}
