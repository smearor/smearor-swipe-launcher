use super::common::with_first_area_manager;
use super::registry::McpResourceHandler;
use crate::host::LauncherHost;

/// Handler for `area://list` — returns all area IDs.
pub struct AreaListHandler;

impl McpResourceHandler for AreaListHandler {
    fn uri_matches(&self, uri: &str) -> bool {
        uri == "area://list"
    }

    fn handle(&self, host: &LauncherHost, _uri: &str) -> Result<String, String> {
        with_first_area_manager(host, |area_manager| {
            let areas = area_manager.list_areas();
            serde_json::to_string(&areas).map_err(|e| e.to_string())
        })
    }
}
