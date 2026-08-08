use gtk4::gdk::Display;
use gtk4::gdk::Monitor;
use gtk4::prelude::Cast;
use gtk4::prelude::DisplayExt;
use gtk4::prelude::ListModelExt;
use tracing::warn;

/// Resolves the monitor for the given index.
/// Falls back to the primary monitor (index 0) if the index is
/// out of bounds or no display is available.
pub fn resolve_monitor(monitor_index: Option<u32>) -> Option<Monitor> {
    let display = Display::default()?;
    let monitors = display.monitors();
    let index = monitor_index.unwrap_or(0);
    monitors
        .item(index)
        .and_then(|m| m.downcast::<Monitor>().ok())
        .or_else(|| monitors.item(0).and_then(|m| m.downcast::<Monitor>().ok()))
}

/// Validates the configured monitor index against the available monitors.
/// Logs a warning if the index is out of bounds.
pub fn validate_monitor_index(monitor_index: Option<u32>, instance_id: &str) {
    let Some(index) = monitor_index else {
        return;
    };
    let Some(display) = Display::default() else {
        return;
    };
    let monitors = display.monitors();
    let count = monitors.n_items();
    if index >= count {
        warn!(
            "Instance '{}': monitor index {} is out of bounds ({} monitor(s) available), \
             falling back to primary monitor",
            instance_id, index, count
        );
    }
}
