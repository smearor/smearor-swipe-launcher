/// Internal monitor event sent from the Hyprland event listener to the worker.
pub enum MonitorEvent {
    /// Monitor was connected.
    Added(String),
    /// Monitor was disconnected.
    Removed(String),
}
