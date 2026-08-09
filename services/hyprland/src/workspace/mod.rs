use crate::event_listener::listener::HyprlandEvent;
use crate::service::HyprlandSharedState;
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
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

/// Internal workspace event sent from the Hyprland event listener to the worker.
pub enum WorkspaceEvent {
    /// Active workspace changed on a monitor.
    Changed(WorkspaceChangedEvent),
}

/// Workspace-specific state for the worker loop.
pub struct WorkspaceState {
    known_workspaces: HashSet<i32>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self {
            known_workspaces: HashSet::new(),
        }
    }
}

/// Register workspace handlers on the shared listener.
pub fn register_handlers(listener: &mut hyprland::event_listener::EventListener, sender: mpsc::UnboundedSender<HyprlandEvent>) {
    let ws_sender = sender.clone();
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
        let _ = ws_sender.send(HyprlandEvent::Workspace(WorkspaceEvent::Changed(event)));
    });
}

/// Process a single workspace event. Atomic, focused, no status logic mixed in.
pub async fn process_event(
    state: &mut WorkspaceState,
    event: WorkspaceEvent,
    core_context: &Option<FfiCoreContext>,
    meta: &PluginMeta,
    enable_workspace_lifecycle: bool,
    shared_state: &Arc<Mutex<HyprlandSharedState>>,
) {
    match event {
        WorkspaceEvent::Changed(mut event) => {
            if let Some(monitor_index) = resolve_monitor_for_workspace(event.workspace_id).await {
                event.monitor_index = monitor_index;
            }

            if enable_workspace_lifecycle && !state.known_workspaces.contains(&event.workspace_id) {
                let lifecycle_event = WorkspaceLifecycleEvent {
                    workspace_name: event.workspace_name.clone(),
                    workspace_id: event.workspace_id,
                    monitor_index: event.monitor_index,
                    lifecycle_type: WorkspaceLifecycleType::Created,
                };
                debug!("Workspace created: {:?}", lifecycle_event);
                if let Ok(mut guard) = shared_state.lock() {
                    guard.latest_workspace_lifecycle = Some(lifecycle_event.clone());
                }
                broadcast_event(core_context, meta, lifecycle_event);
                state.known_workspaces.insert(event.workspace_id);
            }

            debug!("Workspace changed: {:?}", event);
            if let Ok(mut guard) = shared_state.lock() {
                guard.latest_workspace_changed = Some(event.clone());
            }
            broadcast_event(core_context, meta, event);

            if enable_workspace_lifecycle {
                let removed = detect_removed_workspaces(&mut state.known_workspaces).await;
                for lifecycle_event in removed {
                    debug!("Workspace destroyed: {:?}", lifecycle_event);
                    if let Ok(mut guard) = shared_state.lock() {
                        guard.latest_workspace_lifecycle = Some(lifecycle_event.clone());
                    }
                    broadcast_event(core_context, meta, lifecycle_event);
                }
            }
        }
    }
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
