use crate::monitor::event::MonitorEvent;
use crate::service::ensure_hyprland_instance_signature;
use tokio::sync::mpsc;
use tracing::debug;

/// Spawn the Hyprland monitor event listener thread.
///
/// Connects to Hyprland's event socket and listens for monitor added/removed
/// events. Sends `MonitorEvent`s to the provided channel.
pub fn spawn_monitor_listener(event_sender: mpsc::UnboundedSender<MonitorEvent>) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(error) => {
                tracing::error!("Hyprland monitor listener: failed to create runtime: {error}");
                return;
            }
        };

        rt.block_on(async move {
            ensure_hyprland_instance_signature();
            loop {
                let mut listener = hyprland::event_listener::EventListener::new();

                let mon_sender = event_sender.clone();
                listener.add_monitor_added_handler(move |data| {
                    let _ = mon_sender.send(MonitorEvent::Added(data.name));
                });

                let mon_sender2 = event_sender.clone();
                listener.add_monitor_removed_handler(move |data| {
                    let _ = mon_sender2.send(MonitorEvent::Removed(data));
                });

                match listener.start_listener_async().await {
                    Ok(()) => {
                        debug!("Hyprland monitor listener exited cleanly, reconnecting in 5s");
                    }
                    Err(error) => {
                        tracing::error!("Hyprland monitor listener stopped: {error}, reconnecting in 5s");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    });
}
