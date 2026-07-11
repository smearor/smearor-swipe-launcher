use crate::workspace::dbus::GnomeShellEvalProxy;
use crate::workspace::dbus::MutterDisplayConfigProxy;
use smearor_workspace_model::WorkspaceChangedEvent;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

/// Internal event sent from the D-Bus polling thread to the worker thread.
pub enum WorkspaceEvent {
    /// Active workspace changed.
    WorkspaceChanged(WorkspaceChangedEvent),
}

/// Run the GNOME D-Bus polling loop with reconnection support.
///
/// Connects to the session D-Bus, polls `org.gnome.Shell.Eval` for the
/// current workspace at the configured interval, and sends
/// `WorkspaceChangedEvent`s to the worker thread. If the D-Bus connection
/// is lost, retries after 5 seconds.
pub fn run_workspace_polling(sender: mpsc::UnboundedSender<WorkspaceEvent>, poll_interval_ms: u64) {
    let poll_duration = Duration::from_millis(poll_interval_ms);

    loop {
        match poll_workspace_loop(&sender, poll_duration) {
            Ok(()) => {
                debug!("GNOME D-Bus polling loop exited cleanly, reconnecting in 5s");
            }
            Err(error) => {
                warn!("GNOME D-Bus polling loop error: {error}, reconnecting in 5s");
            }
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}

/// Run the polling loop until an error occurs.
fn poll_workspace_loop(sender: &mpsc::UnboundedSender<WorkspaceEvent>, poll_duration: Duration) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        let connection = match zbus::Connection::session().await {
            Ok(conn) => conn,
            Err(error) => {
                warn!("Failed to connect to GNOME session D-Bus: {error}");
                return;
            }
        };

        let shell_proxy = match GnomeShellEvalProxy::new(&connection).await {
            Ok(proxy) => proxy,
            Err(error) => {
                warn!("Failed to create org.gnome.Shell proxy: {error}");
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

        let mut last_workspace: Option<i32> = None;

        debug!("GNOME D-Bus polling started (interval: {:?})", poll_duration);

        loop {
            // Query the current workspace index via org.gnome.Shell.Eval.
            // The JavaScript returns the active workspace index (0-based).
            let js_code = "global.workspace_manager.get_active_workspace_index()";
            let workspace_index = match shell_proxy.eval(js_code).await {
                Ok((success, result)) => {
                    if !success {
                        debug!("org.gnome.Shell.Eval returned failure: {result}");
                        None
                    } else {
                        result.trim().parse::<i32>().ok()
                    }
                }
                Err(error) => {
                    debug!("org.gnome.Shell.Eval error: {error}");
                    None
                }
            };

            if let Some(ws_index) = workspace_index {
                let changed = last_workspace.map_or(true, |prev| prev != ws_index);
                if changed {
                    // Query the workspace name via org.gnome.Shell.Eval.
                    let js_name = format!("global.workspace_manager.get_workspace_by_index({ws_index}).title() || '{ws_index}'");
                    let workspace_name = match shell_proxy.eval(&js_name).await {
                        Ok((success, result)) => {
                            if success {
                                result.trim().to_string()
                            } else {
                                ws_index.to_string()
                            }
                        }
                        Err(_) => ws_index.to_string(),
                    };

                    // Resolve monitor index. GNOME typically has a single
                    // logical monitor per workspace in multi-monitor setups.
                    // We use 0 as the default and try to resolve via
                    // DisplayConfig if available.
                    let monitor_index = resolve_monitor_index(&display_proxy).await.unwrap_or(0);

                    let event = WorkspaceChangedEvent {
                        workspace_name: workspace_name.into(),
                        workspace_id: ws_index,
                        monitor_index,
                    };

                    debug!("GNOME workspace changed: {:?}", event);
                    let _ = sender.send(WorkspaceEvent::WorkspaceChanged(event));

                    last_workspace = Some(ws_index);
                }
            }

            tokio::time::sleep(poll_duration).await;
        }
    });

    Ok(())
}

/// Resolve the monitor index for the active workspace via DisplayConfig.
///
/// GNOME's `GetResources` returns monitor information that can be used to
/// determine the primary monitor index. For now, we return 0 as the default
/// since GNOME workspaces span all monitors by default.
async fn resolve_monitor_index(_display_proxy: &MutterDisplayConfigProxy<'_>) -> Option<u32> {
    // GNOME workspaces typically span all monitors. The monitor index
    // resolution via DisplayConfig is complex and depends on the GNOME
    // version. For the initial implementation, we default to monitor 0.
    // This can be enhanced later by parsing GetResources/GetCurrentState
    // to match the primary monitor connector.
    Some(0)
}
