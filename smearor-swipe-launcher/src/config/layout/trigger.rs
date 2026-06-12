use serde::Deserialize;

/// Defines the trigger condition for switching to a specific layout profile
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum LayoutTrigger {
    /// Default layout, used when no other trigger matches
    Default,
    /// Trigger based on monitor name
    Monitor(String),
    /// Trigger based on workspace number
    Workspace(i32),
    /// Trigger based on both monitor and workspace
    MonitorWorkspace { monitor: String, workspace: i32 },
}
