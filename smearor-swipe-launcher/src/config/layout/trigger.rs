use serde::Deserialize;

/// Defines the trigger condition for switching to a specific layout profile
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum LayoutTrigger {
    /// Default layout, used when no other trigger matches
    Default,
    /// Trigger based on monitor name (connector name, e.g. "DP-1")
    Monitor(String),
    /// Trigger based on monitor index (0-based, matching GDK display order)
    MonitorIndex(u32),
    /// Trigger based on workspace number
    Workspace(i32),
    /// Trigger based on both monitor name and workspace
    MonitorWorkspace { monitor: String, workspace: i32 },
    /// Trigger based on both monitor index and workspace
    MonitorIndexWorkspace { monitor: u32, workspace: i32 },
}
