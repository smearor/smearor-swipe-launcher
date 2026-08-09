use crate::monitor::MonitorEvent;
use crate::status::StatusEvent;
use crate::workspace::WorkspaceEvent;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// Maximum number of fast reconnect attempts before switching to slow backoff.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// Unified event dispatched from the consolidated listener to the worker.
pub enum HyprlandEvent {
    /// Workspace-related event (compositor-unified).
    Workspace(WorkspaceEvent),
    /// Monitor-related event (compositor-unified).
    Monitor(MonitorEvent),
    /// Hyprland-specific status event.
    Status(StatusEvent),
}

/// Spawn the consolidated Hyprland event listener thread.
///
/// All event handlers (workspace, monitor, Hyprland-specific) are registered on a
/// single `EventListener` instance. This avoids multiple socket connections to Hyprland.
/// The listener dispatches raw events to a unified worker via a single channel.
pub fn spawn_event_listener(
    event_sender: mpsc::UnboundedSender<HyprlandEvent>,
    enable_workspace_tracking: bool,
    enable_monitor_events: bool,
    enable_status_events: bool,
) {
    std::thread::spawn(move || {
        debug!("Hyprland event listener thread starting");
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(err) => {
                error!("Hyprland event listener: failed to create runtime: {err}");
                return;
            }
        };

        rt.block_on(async move {
            crate::service::ensure_hyprland_instance_signature();
            debug!(
                "Hyprland event listener: starting event loop with workspace_tracking={}, monitor_events={}, status_events={}",
                enable_workspace_tracking, enable_monitor_events, enable_status_events
            );
            let mut reconnect_attempts: u32 = 0;
            loop {
                let mut listener = hyprland::event_listener::EventListener::new();

                if enable_workspace_tracking {
                    crate::workspace::register_handlers(&mut listener, event_sender.clone());
                }
                if enable_monitor_events {
                    crate::monitor::register_handlers(&mut listener, event_sender.clone());
                }
                if enable_status_events {
                    let (status_sender, mut status_receiver) = mpsc::unbounded_channel::<StatusEvent>();
                    crate::status::register_handlers(&mut listener, status_sender);
                    let forward_sender = event_sender.clone();
                    tokio::spawn(async move {
                        while let Some(status_event) = status_receiver.recv().await {
                            let _ = forward_sender.send(HyprlandEvent::Status(status_event));
                        }
                    });
                }

                match listener.start_listener_async().await {
                    Ok(()) => {
                        reconnect_attempts = 0;
                        debug!("Hyprland event listener exited cleanly, reconnecting in 5s");
                    }
                    Err(err) => {
                        reconnect_attempts += 1;
                        if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                            trace!("Hyprland event listener: {} fast retries exhausted, switching to 30s backoff", reconnect_attempts);
                            tokio::time::sleep(Duration::from_secs(30)).await;
                            continue;
                        }
                        error!("Hyprland event listener stopped: {err}, reconnecting in 5s (attempt {reconnect_attempts}/{MAX_RECONNECT_ATTEMPTS})");
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    });
}
