use smearor_hyprland_model::GroupEvent;
use smearor_hyprland_model::HyprlandStateMessage;
use smearor_hyprland_model::LayerEvent;
use smearor_hyprland_model::MonitorEntry;
use smearor_hyprland_model::SystemEvent;
use smearor_hyprland_model::VersionResponse;
use smearor_hyprland_model::WindowEntry;
use smearor_hyprland_model::WindowEvent;
use smearor_hyprland_model::WorkspaceEvent;
use smearor_model_compositor::MonitorChangedEvent;
use smearor_model_compositor::WorkspaceChangedEvent;
use smearor_model_compositor::WorkspaceLifecycleEvent;
use smearor_model_compositor::WorkspaceSnapshotMessage;

/// Hyprland service plugin.
/// Shared state for MCP resource queries — caches latest events and snapshots.
#[derive(Default)]
pub struct HyprlandSharedState {
    /// Last known Hyprland state (updated on state requests).
    pub last_state: Option<HyprlandStateMessage>,
    /// Latest workspace snapshot (updated on snapshot broadcasts).
    pub workspace_snapshot: Option<WorkspaceSnapshotMessage>,
    /// Latest workspace changed event.
    pub latest_workspace_changed: Option<WorkspaceChangedEvent>,
    /// Latest workspace lifecycle event.
    pub latest_workspace_lifecycle: Option<WorkspaceLifecycleEvent>,
    /// Latest monitor changed event.
    pub latest_monitor_changed: Option<MonitorChangedEvent>,
    /// Latest window status event.
    pub latest_window_event: Option<WindowEvent>,
    /// Latest workspace status event.
    pub latest_workspace_event: Option<WorkspaceEvent>,
    /// Latest group status event.
    pub latest_group_event: Option<GroupEvent>,
    /// Latest layer status event.
    pub latest_layer_event: Option<LayerEvent>,
    /// Latest system status event.
    pub latest_system_event: Option<SystemEvent>,
    /// Latest window list (updated on windows requests).
    pub last_windows: Option<Vec<WindowEntry>>,
    /// Latest monitors list (updated on monitors requests and at service startup).
    pub last_monitors: Option<Vec<MonitorEntry>>,
    /// Latest Hyprland version info (fetched at service startup).
    pub last_version: Option<VersionResponse>,
}
