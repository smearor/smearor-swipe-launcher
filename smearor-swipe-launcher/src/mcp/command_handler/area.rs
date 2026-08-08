use crate::host::LauncherHost;
use smearor_mcp_server::McpCommand;

use super::common::with_first_area_manager;

/// Handle area-related commands (open, close, focus, list, toggle, config).
pub(crate) fn handle_area_command(host: &LauncherHost, command: McpCommand) {
    match command {
        McpCommand::OpenArea(cmd) => {
            let result = with_first_area_manager(host, |area_manager| {
                area_manager
                    .ensure_area(&cmd.params.area_id)
                    .map(|_| format!("Area {} opened", cmd.params.area_id))
            });
            let _ = cmd.response.send(result);
        }
        McpCommand::OpenTransientArea(cmd) => {
            let result = with_first_area_manager(host, |area_manager| {
                let area_config = area_manager
                    .config()
                    .get_area_config(&cmd.params.area_id)
                    .ok_or_else(|| format!("Area {} not found in config", cmd.params.area_id))?
                    .clone();
                let sender_id = area_manager.find_sender_id_for_transient(cmd.params.source_area_id.as_deref());
                area_manager
                    .add_transient_area(&cmd.params.area_id, area_config, sender_id.as_deref())
                    .map_err(|e| format!("Failed to open transient area {}: {}", cmd.params.area_id, e))?;
                Ok(format!("Transient area {} opened", cmd.params.area_id))
            });
            let _ = cmd.response.send(result);
        }
        McpCommand::CloseArea(cmd) => {
            let result = with_first_area_manager(host, |area_manager| {
                area_manager
                    .remove_area(&cmd.params.area_id)
                    .map_err(|e| format!("Failed to close area {}: {}", cmd.params.area_id, e))?;
                Ok(format!("Area {} closed", cmd.params.area_id))
            });
            let _ = cmd.response.send(result);
        }
        McpCommand::FocusArea(cmd) => {
            let result = with_first_area_manager(host, |area_manager| {
                area_manager.focus(&cmd.params.area_id).map(|_| format!("Area {} focused", cmd.params.area_id))
            });
            let _ = cmd.response.send(result);
        }
        McpCommand::ListAreas(cmd) => {
            let result = with_first_area_manager(host, |area_manager| {
                let areas = area_manager.list_areas();
                serde_json::to_string(&areas).map_err(|e| e.to_string())
            });
            let _ = cmd.response.send(result);
        }
        McpCommand::ListAllAreas(cmd) => {
            let result = with_first_area_manager(host, |area_manager| {
                let areas = area_manager.list_all_areas();
                serde_json::to_string(&areas).map_err(|e| e.to_string())
            });
            let _ = cmd.response.send(result);
        }
        McpCommand::ToggleArea(cmd) => {
            let result = with_first_area_manager(host, |area_manager| {
                area_manager.toggle(&cmd.params.area_id).map(|_| format!("Area {} toggled", cmd.params.area_id))
            });
            let _ = cmd.response.send(result);
        }
        McpCommand::GetAreaConfig(cmd) => {
            let result = with_first_area_manager(host, |area_manager| {
                let config = area_manager.get_area_config(&cmd.params.area_id)?;
                let mut config_value = serde_json::to_value(&config).map_err(|e| e.to_string())?;
                if let Some(plugins) = config_value.get_mut("plugins").and_then(|v| v.as_array_mut()) {
                    for plugin_value in plugins.iter_mut() {
                        if let Some(plugin_id) = plugin_value.get("id").and_then(|v| v.as_str()) {
                            if let Some(plugin_config) = area_manager.config().get_plugin_config(plugin_id) {
                                if let Some(plugin_object) = plugin_value.as_object_mut() {
                                    plugin_object.insert("config".to_string(), plugin_config.clone());
                                }
                            }
                        }
                    }
                }
                serde_json::to_string(&config_value).map_err(|e| e.to_string())
            });
            let _ = cmd.response.send(result);
        }
        _ => unreachable!("handle_area_command received non-area command"),
    }
}
