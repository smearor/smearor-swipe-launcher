use crate::workspace::dbus::GnomeShellEvalProxy;
use crate::workspace::dbus::MutterDisplayConfigProxy;
use smearor_model_compositor::WorkspaceChangedEvent;
use smearor_model_compositor::WorkspaceLifecycleEvent;
use smearor_model_compositor::WorkspaceLifecycleType;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

/// Internal workspace event sent from the D-Bus polling thread to the worker.
pub enum WorkspaceEvent {
    /// Active workspace changed.
    Changed(WorkspaceChangedEvent),
    /// Workspace created or destroyed.
    Lifecycle(WorkspaceLifecycleEvent),
}

/// Run the GNOME D-Bus polling loop with reconnection support.
///
/// Connects to the session D-Bus, polls `org.gnome.Shell.Eval` for the
/// current workspace at the configured interval, and sends
/// `WorkspaceChangedEvent`s to the worker thread. If the D-Bus connection
/// is lost, retries after 5 seconds.
pub fn run_workspace_polling(sender: mpsc::UnboundedSender<WorkspaceEvent>, poll_interval_ms: u64, enable_workspace_lifecycle: bool) {
    let poll_duration = Duration::from_millis(poll_interval_ms);

    loop {
        match poll_workspace_loop(&sender, poll_duration, enable_workspace_lifecycle) {
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
fn poll_workspace_loop(
    sender: &mpsc::UnboundedSender<WorkspaceEvent>,
    poll_duration: Duration,
    enable_workspace_lifecycle: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        let mut last_workspace_count: Option<i32> = None;

        debug!("GNOME D-Bus polling started (interval: {:?})", poll_duration);

        loop {
            // Query the current workspace index via org.gnome.Shell.Eval.
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

                    let monitor_index = resolve_monitor_index(&display_proxy).await.unwrap_or(0);

                    let event = WorkspaceChangedEvent {
                        workspace_name: workspace_name.into(),
                        workspace_id: ws_index,
                        monitor_index,
                    };

                    debug!("GNOME workspace changed: {:?}", event);
                    let _ = sender.send(WorkspaceEvent::Changed(event));

                    last_workspace = Some(ws_index);
                }
            }

            // Workspace lifecycle detection via workspace count polling.
            if enable_workspace_lifecycle {
                let js_count = "global.workspace_manager.get_n_workspaces()";
                let count = shell_proxy
                    .eval(js_count)
                    .await
                    .ok()
                    .and_then(|(success, result)| if success { result.trim().parse::<i32>().ok() } else { None });

                if let Some(current_count) = count {
                    if let Some(prev_count) = last_workspace_count {
                        if current_count > prev_count {
                            for i in prev_count..current_count {
                                let js_name = format!("global.workspace_manager.get_workspace_by_index({i}).title() || '{i}'");
                                let name = shell_proxy
                                    .eval(&js_name)
                                    .await
                                    .ok()
                                    .and_then(|(success, result)| if success { Some(result.trim().to_string()) } else { None })
                                    .unwrap_or_else(|| i.to_string());

                                let event = WorkspaceLifecycleEvent {
                                    workspace_name: name.into(),
                                    workspace_id: i,
                                    monitor_index: 0,
                                    lifecycle_type: WorkspaceLifecycleType::Created,
                                };
                                debug!("GNOME workspace created: {:?}", event);
                                let _ = sender.send(WorkspaceEvent::Lifecycle(event));
                            }
                        } else if current_count < prev_count {
                            for i in current_count..prev_count {
                                let event = WorkspaceLifecycleEvent {
                                    workspace_name: i.to_string().into(),
                                    workspace_id: i,
                                    monitor_index: 0,
                                    lifecycle_type: WorkspaceLifecycleType::Destroyed,
                                };
                                debug!("GNOME workspace destroyed: {:?}", event);
                                let _ = sender.send(WorkspaceEvent::Lifecycle(event));
                            }
                        }
                    }
                    last_workspace_count = Some(current_count);
                }
            }

            tokio::time::sleep(poll_duration).await;
        }
    });

    Ok(())
}

/// Resolve the monitor index for the active workspace via DisplayConfig.
///
/// GNOME's `GetResources` returns logical monitor information. The primary
/// monitor is marked with `primary = true` in the logical monitors list.
async fn resolve_monitor_index(display_proxy: &MutterDisplayConfigProxy<'_>) -> Option<u32> {
    let (_serial, _monitors, logical_monitors, _props) = display_proxy.get_resources().await.ok()?;

    for (index, logical) in logical_monitors.iter().enumerate() {
        // Each logical monitor is a Vec containing a single tuple:
        // (x: i32, y: i32, scale: u32, primary: bool, monitors: Vec<...>).
        if let Some((_x, _y, _scale, primary, _monitors)) = logical.first() {
            if *primary {
                return Some(index as u32);
            }
        }
    }

    if !logical_monitors.is_empty() {
        return Some(0);
    }

    None
}

/// Spawn the workspace event worker thread that broadcasts workspace events
/// to the launcher core.
pub fn spawn_workspace_worker(mut event_receiver: mpsc::UnboundedReceiver<WorkspaceEvent>, core_context: Option<FfiCoreContext>, meta: PluginMeta) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(error) => {
                tracing::error!("GNOME workspace worker: failed to create runtime: {error}");
                return;
            }
        };

        rt.block_on(async move {
            while let Some(event) = event_receiver.recv().await {
                match event {
                    WorkspaceEvent::Changed(event) => {
                        debug!("Broadcasting workspace changed event: {:?}", event);
                        broadcast_event(&core_context, &meta, event);
                    }
                    WorkspaceEvent::Lifecycle(event) => {
                        debug!("Broadcasting workspace lifecycle event: {:?}", event);
                        broadcast_event(&core_context, &meta, event);
                    }
                }
            }
        });
    });
}

/// Broadcast an event to all launcher instances via the core context.
fn broadcast_event<T>(core_context: &Option<FfiCoreContext>, meta: &PluginMeta, event: T)
where
    T: Clone + MessageTopic + TypedMessage,
{
    let Some(ctx) = core_context else {
        return;
    };
    let broadcaster = MessageBroadcasterInner {
        meta: meta.clone(),
        core_context: Some(*ctx),
    };
    broadcaster.broadcast_message_to_topic(event);
}
