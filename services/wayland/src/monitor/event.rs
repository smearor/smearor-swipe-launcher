use smearor_model_compositor::MonitorChangedEvent;

/// Internal monitor event sent from the Wayland listener thread to the worker.
pub enum MonitorEvent {
    /// Monitor connected or disconnected.
    Changed(MonitorChangedEvent),
}
