use smearor_model_compositor::WorkspaceChangedEvent;

/// Internal workspace event sent from the Hyprland event listener to the worker.
pub enum WorkspaceEvent {
    /// Active workspace changed on a monitor.
    Changed(WorkspaceChangedEvent),
}
