use super::event::MonitorEvent;
use crate::event_listener::listener::HyprlandEvent;
use tokio::sync::mpsc;

/// Register monitor handlers on the shared listener.
pub fn register_handlers(listener: &mut hyprland::event_listener::EventListener, sender: mpsc::UnboundedSender<HyprlandEvent>) {
    let mon_sender = sender.clone();
    listener.add_monitor_added_handler(move |data| {
        let _ = mon_sender.send(HyprlandEvent::Monitor(MonitorEvent::Added(data.name)));
    });

    let mon_sender2 = sender.clone();
    listener.add_monitor_removed_handler(move |data| {
        let _ = mon_sender2.send(HyprlandEvent::Monitor(MonitorEvent::Removed(data)));
    });
}
