use crate::monitor::event::MonitorEvent;
use crate::monitor::event::MonitorInfo;
use crate::workspace::dbus::MutterDisplayConfigProxy;
use smearor_model_compositor::MonitorChangeType;
use smearor_model_compositor::MonitorChangedEvent;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

/// Run the GNOME D-Bus monitor polling loop with reconnection support.
///
/// Polls `org.gnome.Mutter.DisplayConfig` at the configured interval and
/// detects monitor connect/disconnect events by comparing the monitor list
/// between polls.
pub fn run_monitor_polling(sender: mpsc::UnboundedSender<MonitorEvent>, poll_interval_ms: u64) {
    let poll_duration = std::time::Duration::from_millis(poll_interval_ms);

    loop {
        match poll_monitor_loop(&sender, poll_duration) {
            Ok(()) => {
                debug!("GNOME monitor polling loop exited cleanly, reconnecting in 5s");
            }
            Err(error) => {
                warn!("GNOME monitor polling loop error: {error}, reconnecting in 5s");
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

/// Run the monitor polling loop until an error occurs.
fn poll_monitor_loop(sender: &mpsc::UnboundedSender<MonitorEvent>, poll_duration: std::time::Duration) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        let connection = match zbus::Connection::session().await {
            Ok(conn) => conn,
            Err(error) => {
                warn!("Failed to connect to GNOME session D-Bus: {error}");
                return;
            }
        };

        let display_proxy = match MutterDisplayConfigProxy::new(&connection).await {
            Ok(proxy) => proxy,
            Err(error) => {
                warn!("Failed to create org.gnome.Mutter.DisplayConfig proxy: {error}");
                return;
            }
        };

        let mut previous_monitors: Vec<MonitorInfo> = query_monitors(&display_proxy).await.unwrap_or_default();

        debug!("GNOME monitor polling started (interval: {:?})", poll_duration);

        loop {
            let current_monitors = query_monitors(&display_proxy).await.unwrap_or_default();
            if current_monitors != previous_monitors {
                let changes = detect_monitor_changes(&previous_monitors, &current_monitors);
                for change in changes {
                    debug!("GNOME monitor changed: {:?}", change);
                    let _ = sender.send(MonitorEvent::Changed(change));
                }
                previous_monitors = current_monitors;
            }

            tokio::time::sleep(poll_duration).await;
        }
    });

    Ok(())
}

/// Query the current monitor list from DisplayConfig.
async fn query_monitors(display_proxy: &MutterDisplayConfigProxy<'_>) -> Result<Vec<MonitorInfo>, zbus::Error> {
    let (_serial, monitors, _logical, _props) = display_proxy.get_resources().await?;

    let mut result = Vec::new();
    for (index, (connector, _modes)) in monitors.into_iter().enumerate() {
        result.push(MonitorInfo {
            index: index as u32,
            connector,
        });
    }
    Ok(result)
}

/// Detect monitor changes between two snapshots.
fn detect_monitor_changes(previous: &[MonitorInfo], current: &[MonitorInfo]) -> Vec<MonitorChangedEvent> {
    let mut events = Vec::new();

    for mon in current {
        if !previous.iter().any(|p| p.connector == mon.connector) {
            events.push(MonitorChangedEvent {
                monitor_index: mon.index,
                connector_name: mon.connector.clone().into(),
                change_type: MonitorChangeType::Connected,
            });
        }
    }

    for mon in previous {
        if !current.iter().any(|c| c.connector == mon.connector) {
            events.push(MonitorChangedEvent {
                monitor_index: mon.index,
                connector_name: mon.connector.clone().into(),
                change_type: MonitorChangeType::Disconnected,
            });
        }
    }

    events
}
