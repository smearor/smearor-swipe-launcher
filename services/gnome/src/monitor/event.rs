use smearor_model_compositor::MonitorChangedEvent;

/// Internal monitor event sent from the D-Bus polling thread to the worker.
pub enum MonitorEvent {
    /// Monitor connected or disconnected.
    Changed(MonitorChangedEvent),
}

/// Information about a physical monitor queried from DisplayConfig.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    /// Sequential index in the monitor list.
    pub index: u32,
    /// Connector name (e.g. "DP-1", "HDMI-A-1").
    pub connector: String,
}
