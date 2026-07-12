use crate::workspace::dbus::GnomeShellEvalProxy;
use crate::workspace::dbus::GnomeShellIntrospectProxy;
use crate::workspace::dbus::MutterDisplayConfigProxy;
use crate::workspace::gsettings;
use smearor_model_compositor::WorkspaceChangedEvent;
use smearor_model_compositor::WorkspaceLifecycleEvent;
use smearor_model_compositor::WorkspaceLifecycleType;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;
use zbus::zvariant::OwnedValue;
use zbus::zvariant::Value;

/// Internal workspace event sent from the D-Bus polling thread to the worker.
pub enum WorkspaceEvent {
    /// Active workspace changed.
    Changed(WorkspaceChangedEvent),
    /// Workspace created or destroyed.
    Lifecycle(WorkspaceLifecycleEvent),
}

/// Run the GNOME D-Bus polling loop with reconnection support.
///
/// Connects to the session D-Bus, polls `org.gnome.Shell.Introspect.GetWindows`
/// for the current workspace at the configured interval, and sends
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

        let introspect_proxy = match GnomeShellIntrospectProxy::new(&connection).await {
            Ok(proxy) => Some(proxy),
            Err(error) => {
                warn!("Failed to create org.gnome.Shell.Introspect proxy: {error}");
                None
            }
        };

        let shell_eval_proxy = match GnomeShellEvalProxy::new(&connection).await {
            Ok(proxy) => Some(proxy),
            Err(error) => {
                warn!("Failed to create org.gnome.Shell proxy: {error}");
                None
            }
        };

        let display_proxy = match MutterDisplayConfigProxy::new(&connection).await {
            Ok(proxy) => Some(proxy),
            Err(error) => {
                warn!("Failed to create org.gnome.Mutter.DisplayConfig proxy: {error}");
                None
            }
        };

        // Determine which data source to use for workspace tracking.
        // Priority: Introspect.GetWindows > Shell.Eval > GSettings-only (active=0)
        let mut introspect_available = introspect_proxy.is_some();
        let mut eval_available = shell_eval_proxy.is_some();

        // Do an initial probe to check if Introspect is actually accessible.
        if let Some(ref proxy) = introspect_proxy {
            match proxy.get_windows().await {
                Ok(_) => {
                    debug!("GNOME workspace tracking: using org.gnome.Shell.Introspect");
                }
                Err(error) => {
                    warn!("GNOME workspace tracking: Introspect.GetWindows blocked ({error}), falling back to Shell.Eval + GSettings");
                    introspect_available = false;
                }
            }
        }

        // If Introspect is not available, probe Shell.Eval.
        if !introspect_available {
            if let Some(ref proxy) = shell_eval_proxy {
                match proxy.eval("global.workspace_manager.get_active_workspace_index()").await {
                    Ok((true, result)) => {
                        debug!("GNOME workspace tracking: using Shell.Eval (active={})", result.trim());
                    }
                    Ok((false, _)) => {
                        warn!("GNOME workspace tracking: Shell.Eval returned failure (enable unsafe mode for full functionality)");
                        eval_available = false;
                    }
                    Err(error) => {
                        warn!("GNOME workspace tracking: Shell.Eval error ({error}), falling back to GSettings-only");
                        eval_available = false;
                    }
                }
            } else {
                eval_available = false;
            }
        }

        if !introspect_available && !eval_available {
            warn!("GNOME workspace tracking: both Introspect and Eval are blocked. Using GSettings-only mode (active workspace detection not available). Enable GNOME Shell unsafe mode for full functionality.");
        }

        let mut last_workspace: Option<i32> = None;
        let mut last_workspace_count: Option<i32> = None;

        debug!("GNOME D-Bus polling started (interval: {:?}, introspect={}, eval={})", poll_duration, introspect_available, eval_available);

        loop {
            let (active_workspace, max_workspace) = if introspect_available {
                if let Some(ref proxy) = introspect_proxy {
                    match proxy.get_windows().await {
                        Ok(windows) => analyze_windows(&windows),
                        Err(error) => {
                            debug!("org.gnome.Shell.Introspect.GetWindows error: {error}");
                            // Mark as unavailable for future iterations
                            introspect_available = false;
                            (None, 0)
                        }
                    }
                } else {
                    (None, 0)
                }
            } else if eval_available {
                if let Some(ref proxy) = shell_eval_proxy {
                    let active = eval_active_workspace(proxy).await;
                    let count = if gsettings::is_dynamic_workspaces() {
                        active.map(|a| a + 1).unwrap_or(1)
                    } else {
                        gsettings::read_workspace_count()
                    };
                    (active, count.saturating_sub(1))
                } else {
                    (None, 0)
                }
            } else {
                // GSettings-only mode: can't detect active workspace
                let count = gsettings::read_workspace_count();
                (Some(0), count.saturating_sub(1))
            };

            if let Some(ws_index) = active_workspace {
                let changed = last_workspace.map_or(true, |prev| prev != ws_index);
                if changed {
                    let workspace_name = resolve_workspace_name(ws_index, max_workspace);
                    let monitor_index = if let Some(ref display_proxy) = display_proxy {
                        resolve_monitor_index(display_proxy).await.unwrap_or(0)
                    } else {
                        0
                    };

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
                let current_count = if gsettings::is_dynamic_workspaces() {
                    max_workspace + 1
                } else {
                    gsettings::read_workspace_count()
                };

                if let Some(prev_count) = last_workspace_count {
                    if current_count > prev_count {
                        for i in prev_count..current_count {
                            let name = gsettings::read_workspace_names(current_count)
                                .get(i as usize)
                                .cloned()
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

            tokio::time::sleep(poll_duration).await;
        }
    });

    Ok(())
}

/// Analyze the Introspect window map to find the active workspace and max workspace ID.
///
/// Returns `(active_workspace, max_workspace)` where `active_workspace` is the
/// workspace index of the focused window (or `None` if no window has focus),
/// and `max_workspace` is the highest workspace ID seen across all windows.
fn analyze_windows(windows: &HashMap<String, HashMap<String, OwnedValue>>) -> (Option<i32>, i32) {
    let mut active_workspace: Option<i32> = None;
    let mut max_workspace: i32 = 0;

    for props in windows.values() {
        if let Some(workspace_value) = props.get("workspace") {
            let ws_id = extract_i32(workspace_value);
            if ws_id > max_workspace {
                max_workspace = ws_id;
            }
        }

        if let Some(focus_value) = props.get("has-focus") {
            let has_focus = matches!(&**focus_value, Value::Bool(true));
            if has_focus {
                if let Some(workspace_value) = props.get("workspace") {
                    active_workspace = Some(extract_i32(workspace_value));
                }
            }
        }
    }

    (active_workspace, max_workspace)
}

/// Extract an i32 from a zvariant OwnedValue.
fn extract_i32(value: &OwnedValue) -> i32 {
    match &**value {
        Value::I32(v) => *v,
        Value::I64(v) => *v as i32,
        Value::U32(v) => *v as i32,
        Value::U64(v) => *v as i32,
        _ => value.to_string().trim().parse::<i32>().ok().unwrap_or(0),
    }
}

/// Query the active workspace index via Shell.Eval.
///
/// Returns `None` if Eval is blocked or fails.
async fn eval_active_workspace(proxy: &GnomeShellEvalProxy<'_>) -> Option<i32> {
    match proxy.eval("global.workspace_manager.get_active_workspace_index()").await {
        Ok((true, result)) => result.trim().parse::<i32>().ok(),
        Ok((false, _)) => None,
        Err(_) => None,
    }
}

/// Resolve a workspace name from GSettings or fall back to the index.
fn resolve_workspace_name(workspace_id: i32, max_workspace: i32) -> String {
    let count = max_workspace + 1;
    let names = gsettings::read_workspace_names(count);
    names.get(workspace_id as usize).cloned().unwrap_or_else(|| workspace_id.to_string())
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
