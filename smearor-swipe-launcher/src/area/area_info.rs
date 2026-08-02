/// Snapshot of an area's current state for external consumers such as the
/// MCP server.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AreaInfo {
    pub area_id: String,
    pub visible: bool,
    pub focused: bool,
    pub position: String,
    pub active: bool,
}

/// Information about all configured areas, including those not yet opened.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AllAreaInfo {
    pub area_id: String,
    pub visible: bool,
    pub active: bool,
    pub area_type: String,
}
