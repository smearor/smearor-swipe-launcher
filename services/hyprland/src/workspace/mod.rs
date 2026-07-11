use hyprland::shared::HyprData;
use smearor_model_compositor::WorkspaceChangedEvent;
use smearor_model_compositor::WorkspaceLifecycleEvent;
use smearor_model_compositor::WorkspaceLifecycleType;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::collections::HashSet;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

/// Internal workspace event sent from the Hyprland event listener to the worker.
pub enum WorkspaceEvent {
    /// Active workspace changed on a monitor.
    Changed(WorkspaceChangedEvent),
}

/// Spawn the Hyprland workspace event listener thread.
///
/// Connects to Hyprland's event socket and listens for workspace change events.
/// Sends `WorkspaceEvent`s to the provided channel.
pub fn spawn_workspace_listener(event_sender: mpsc::UnboundedSender<WorkspaceEvent>) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(error) => {
                tracing::error!("Hyprland workspace listener: failed to create runtime: {error}");
                return;
            }
        };

        rt.block_on(async move {
            loop {
                let mut listener = hyprland::event_listener::EventListener::new();

                let ws_sender = event_sender.clone();
                listener.add_workspace_changed_handler(move |workspace_data| {
                    let workspace_name = match &workspace_data.name {
                        hyprland::shared::WorkspaceType::Regular(name) => name.clone(),
                        hyprland::shared::WorkspaceType::Special(name) => name.clone().unwrap_or_default(),
                    };
                    let event = WorkspaceChangedEvent {
                        workspace_name: workspace_name.into(),
                        workspace_id: workspace_data.id,
                        monitor_index: 0,
                    };
                    let _ = ws_sender.send(WorkspaceEvent::Changed(event));
                });

                match listener.start_listener_async().await {
                    Ok(()) => {
                        debug!("Hyprland workspace listener exited cleanly, reconnecting in 5s");
                    }
                    Err(error) => {
                        tracing::error!("Hyprland workspace listener stopped: {error}, reconnecting in 5s");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    });
}

/// Spawn the workspace event worker thread that processes events and broadcasts
/// to the launcher core.
pub fn spawn_workspace_worker(
    mut event_receiver: mpsc::UnboundedReceiver<WorkspaceEvent>,
    core_context: Option<FfiCoreContext>,
    meta: PluginMeta,
    enable_workspace_lifecycle: bool,
) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(error) => {
                tracing::error!("Hyprland workspace worker: failed to create runtime: {error}");
                return;
            }
        };

        rt.block_on(async move {
            let mut known_workspaces: HashSet<i32> = HashSet::new();

            while let Some(event) = event_receiver.recv().await {
                match event {
                    WorkspaceEvent::Changed(mut event) => {
                        if let Some(monitor_index) = resolve_monitor_for_workspace(event.workspace_id).await {
                            event.monitor_index = monitor_index;
                        }

                        if enable_workspace_lifecycle && !known_workspaces.contains(&event.workspace_id) {
                            let lifecycle_event = WorkspaceLifecycleEvent {
                                workspace_name: event.workspace_name.clone(),
                                workspace_id: event.workspace_id,
                                monitor_index: event.monitor_index,
                                lifecycle_type: WorkspaceLifecycleType::Created,
                            };
                            debug!("Workspace created: {:?}", lifecycle_event);
                            broadcast_event(&core_context, &meta, lifecycle_event);
                            known_workspaces.insert(event.workspace_id);
                        }

                        debug!("Workspace changed: {:?}", event);
                        broadcast_event(&core_context, &meta, event);

                        if enable_workspace_lifecycle {
                            let removed = detect_removed_workspaces(&mut known_workspaces).await;
                            for lifecycle_event in removed {
                                debug!("Workspace destroyed: {:?}", lifecycle_event);
                                broadcast_event(&core_context, &meta, lifecycle_event);
                            }
                        }
                    }
                }
            }
        });
    });
}

/// Query `hyprctl monitors` to find which monitor index has the given workspace active.
async fn resolve_monitor_for_workspace(workspace_id: i32) -> Option<u32> {
    let monitors = match hyprland::data::Monitors::get() {
        Ok(monitors) => monitors,
        Err(error) => {
            warn!("Failed to query monitors for workspace {workspace_id}: {error}");
            return None;
        }
    };
    for monitor in monitors {
        if monitor.active_workspace.id == workspace_id {
            return Some(monitor.id as u32);
        }
    }
    None
}

/// Detect workspaces that have been removed since the last check.
async fn detect_removed_workspaces(known: &mut HashSet<i32>) -> Vec<WorkspaceLifecycleEvent> {
    let current = match hyprland::data::Workspaces::get() {
        Ok(workspaces) => workspaces,
        Err(_) => return Vec::new(),
    };

    let current_ids: HashSet<i32> = current.iter().map(|ws| ws.id).collect();
    let removed: Vec<WorkspaceLifecycleEvent> = known
        .difference(&current_ids)
        .map(|id| WorkspaceLifecycleEvent {
            workspace_name: id.to_string().into(),
            workspace_id: *id,
            monitor_index: 0,
            lifecycle_type: WorkspaceLifecycleType::Destroyed,
        })
        .collect();
    known.retain(|id| current_ids.contains(id));
    removed
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
        core_context: Some(ctx.clone()),
    };
    broadcaster.broadcast_message_to_topic(event);
}
