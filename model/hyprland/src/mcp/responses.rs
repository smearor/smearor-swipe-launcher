use serde::Deserialize;
use serde::Serialize;

/// Active window information in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveWindowEntry {
    /// Window class (application identifier)
    pub class: String,
    /// Window title
    pub title: String,
    /// Workspace ID the window is on
    pub workspace_id: i32,
}

/// Response body for the `hyprland://state` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HyprlandStateResponse {
    /// Active window information, or null if no window is focused
    pub active_window: Option<ActiveWindowEntry>,
    /// Whether the active window is fullscreen
    pub is_fullscreen: bool,
    /// Current keyboard layout name
    pub keyboard_layout: String,
    /// Current submap name
    pub submap: String,
}
