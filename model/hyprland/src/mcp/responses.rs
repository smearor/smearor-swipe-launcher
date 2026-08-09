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

/// A single workspace entry in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    /// The workspace ID (numeric, as reported by the compositor)
    pub workspace_id: i32,
    /// The workspace name or number as a string
    pub workspace_name: String,
    /// The monitor index (0-based) on which the workspace is located
    pub monitor_index: u32,
    /// Whether this workspace is currently active
    pub is_active: bool,
}

/// Response body for the `hyprland://workspace-snapshot` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSnapshotResponse {
    /// All known workspaces, ordered by workspace ID
    pub workspaces: Vec<WorkspaceEntry>,
    /// The currently active workspace ID
    pub active_workspace_id: i32,
    /// The monitor index on which the active workspace is located
    pub active_monitor_index: u32,
}

/// Response body for the `hyprland://workspaces` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspacesResponse {
    /// List of all workspaces
    pub workspaces: Vec<WorkspaceEntry>,
}

/// A single monitor entry in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonitorEntry {
    /// The monitor index (0-based, matching GDK display order)
    pub monitor_index: u32,
    /// The connector name of the monitor (e.g. "HDMI-A-1", "eDP-1")
    pub connector_name: String,
}

/// Response body for the `hyprland://monitors` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonitorsResponse {
    /// List of all monitors
    pub monitors: Vec<MonitorEntry>,
}

/// Response body for the `hyprland://window-status` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowStatusResponse {
    /// Recent window status events
    pub events: Vec<WindowStatusEvent>,
}

/// A window status event in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowStatusEvent {
    /// The event type as a string
    pub event_type: String,
    /// The window class, if available
    pub class: Option<String>,
    /// The window title, if available
    pub title: Option<String>,
    /// The workspace ID, if available
    pub workspace_id: Option<i32>,
}

/// Response body for the `hyprland://workspace-status` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceStatusResponse {
    /// Recent workspace status events
    pub events: Vec<WorkspaceStatusEvent>,
}

/// A workspace status event in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceStatusEvent {
    /// The event type as a string
    pub event_type: String,
    /// The workspace ID, if available
    pub workspace_id: Option<i32>,
    /// The workspace name, if available
    pub workspace_name: Option<String>,
}

/// Response body for the `hyprland://group-status` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupStatusResponse {
    /// Recent group status events
    pub events: Vec<GroupStatusEvent>,
}

/// A group status event in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupStatusEvent {
    /// The event type as a string
    pub event_type: String,
}

/// Response body for the `hyprland://layer-status` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayerStatusResponse {
    /// Recent layer shell status events
    pub events: Vec<LayerStatusEvent>,
}

/// A layer shell status event in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayerStatusEvent {
    /// The event type as a string
    pub event_type: String,
    /// The namespace of the layer surface, if available
    pub namespace: Option<String>,
}

/// Response body for the `hyprland://system-status` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    /// Recent system status events
    pub events: Vec<SystemStatusEvent>,
}

/// A system status event in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemStatusEvent {
    /// The event type as a string
    pub event_type: String,
}
