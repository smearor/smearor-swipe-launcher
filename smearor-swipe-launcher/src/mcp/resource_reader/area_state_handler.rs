use super::common::with_first_area_manager;
use super::registry::McpResourceHandler;
use crate::host::LauncherHost;

/// Handler for `area://<id>/state` — returns the state of a specific area.
pub struct AreaStateHandler;

impl McpResourceHandler for AreaStateHandler {
    fn uri_matches(&self, uri: &str) -> bool {
        uri.starts_with("area://") && uri.ends_with("/state")
    }

    fn handle(&self, host: &LauncherHost, uri: &str) -> Result<String, String> {
        let area_id = uri.trim_start_matches("area://").trim_end_matches("/state");
        with_first_area_manager(host, |area_manager| {
            let areas = area_manager.list_areas();
            let area = areas.into_iter().find(|a| a.area_id == area_id).ok_or(format!("Area {} not found", area_id))?;
            serde_json::to_string(&area).map_err(|e| e.to_string())
        })
    }
}
