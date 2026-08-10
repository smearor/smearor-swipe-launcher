use serde::Deserialize;
use serde::Serialize;

/// Active window information in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveWindowEntry {
    /// Window class (application identifier)
    pub class: String,
    /// Window title
    pub title: String,
    /// Window address as a hex string (e.g. "0x1234567")
    pub address: String,
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
    /// Monitor width in pixels
    pub width: u32,
    /// Monitor height in pixels
    pub height: u32,
    /// Refresh rate in Hz
    pub refresh_rate: f32,
    /// X position in the global coordinate space
    pub x: i32,
    /// Y position in the global coordinate space
    pub y: i32,
    /// Active workspace ID on this monitor
    pub active_workspace_id: i32,
    /// Active workspace name on this monitor
    pub active_workspace_name: String,
    /// Scale factor (e.g. 1.0, 1.5, 2.0)
    pub scale: f32,
    /// Transform as a string ("normal", "90", "180", "270", "flipped", "90_flipped", "180_flipped", "270_flipped")
    pub transform: String,
    /// Whether this monitor is the primary/focused monitor
    pub focused: bool,
    /// DPMS (display power management) status
    pub dpms_status: bool,
    /// Variable refresh rate enabled
    pub vrr: bool,
    /// Whether this monitor is disabled
    pub disabled: bool,
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

/// A single window entry in MCP resource responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowEntry {
    /// The window class (application identifier)
    pub class: String,
    /// The window title
    pub title: String,
    /// The initial window class (set when the window was first created)
    pub initial_class: String,
    /// The initial window title (set when the window was first created)
    pub initial_title: String,
    /// The window address as a hex string (e.g. "0x1234")
    pub address: String,
    /// The workspace ID the window is on
    pub workspace_id: i32,
    /// The monitor ID the window is on (may be None in some cases)
    pub monitor: Option<i128>,
    /// Whether this window is floating
    pub floating: bool,
    /// The fullscreen mode as a string ("none", "maximized", "fullscreen", "maximized_fullscreen")
    pub fullscreen_mode: String,
    /// Whether this window is pinned
    pub pinned: bool,
    /// Whether this window is mapped (visible on screen)
    pub mapped: bool,
    /// Whether this window is running under XWayland
    pub xwayland: bool,
    /// The process ID of the window
    pub pid: i32,
    /// The XDG tag for the window
    pub xdg_tag: String,
    /// The XDG description for the window
    pub xdg_description: String,
    /// Whether this window is the currently focused window
    pub is_active: bool,
}

/// Response body for the `hyprland://windows` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowsResponse {
    /// List of all windows
    pub windows: Vec<WindowEntry>,
}

/// Response body for the `hyprland://version` MCP resource.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct VersionResponse {
    /// Hyprland version tag (e.g. "0.49.0")
    pub tag: String,
    /// Git branch
    pub branch: String,
    /// Git commit hash
    pub commit: String,
    /// Whether dirty (uncommitted changes)
    pub dirty: bool,
    /// Git commit message
    pub commit_message: String,
    /// Git commit date
    pub commit_date: String,
    /// Number of commits at build time
    pub commits: String,
    /// Aquamarine build version
    pub build_aquamarine: String,
    /// Build flags
    pub flags: Vec<String>,
}

/// Response body for the `hyprland_ctl_keyword_get` MCP tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeywordGetResponse {
    /// The keyword name that was queried
    pub keyword: String,
    /// The value as a string (Hyprland returns int/float/str)
    pub value: String,
    /// Whether the keyword was set by the user (vs default)
    pub set: bool,
}
