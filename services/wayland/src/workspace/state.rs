use crate::monitor::MonitorEvent;
use smearor_model_compositor::WorkspaceChangedEvent;
use smearor_model_compositor::WorkspaceLifecycleEvent;
use smearor_model_compositor::WorkspaceLifecycleType;
use std::collections::HashMap;
use std::collections::HashSet;
use tokio::sync::mpsc;
use tracing::debug;
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1;

/// Internal workspace event sent from the Wayland listener thread to the worker thread.
pub enum WorkspaceEvent {
    /// Active workspace changed.
    WorkspaceChanged(WorkspaceChangedEvent),
    /// Workspace created or destroyed.
    WorkspaceLifecycle(WorkspaceLifecycleEvent),
}

/// Information tracked for each workspace handle.
pub struct WorkspaceInfo {
    /// The proxy handle, kept alive to receive events.
    #[allow(dead_code)]
    pub handle: ExtWorkspaceHandleV1,
    /// Human-readable workspace name.
    pub name: String,
    /// Stable workspace id assigned by the compositor.
    pub id: String,
    /// Whether the workspace is currently active.
    pub is_active: bool,
    /// The group this workspace belongs to, if any.
    pub group_id: Option<ObjectId>,
}

/// Information tracked for each workspace group handle.
pub struct GroupInfo {
    /// The proxy handle, kept alive to receive events.
    #[allow(dead_code)]
    pub handle: ExtWorkspaceGroupHandleV1,
    /// Outputs assigned to this group.
    pub output_ids: Vec<ObjectId>,
}

/// Wayland event listener state, used as the dispatch target for the event queue.
pub struct WaylandState {
    /// Channel for sending workspace events to the worker thread.
    pub workspace_sender: mpsc::UnboundedSender<WorkspaceEvent>,
    /// Channel for sending monitor events to the worker thread.
    pub monitor_sender: mpsc::UnboundedSender<MonitorEvent>,
    /// The workspace manager proxy, kept alive to receive events.
    pub manager: Option<ExtWorkspaceManagerV1>,
    /// All known workspaces keyed by their object id.
    pub workspaces: HashMap<ObjectId, WorkspaceInfo>,
    /// All known workspace groups keyed by their object id.
    pub groups: HashMap<ObjectId, GroupInfo>,
    /// All bound outputs, kept alive.
    pub outputs: HashMap<ObjectId, WlOutput>,
    /// Maps output object id to a sequential monitor index.
    pub output_to_index: HashMap<ObjectId, u32>,
    /// Maps global name (u32) to output ObjectId for removal handling.
    pub global_name_to_output: HashMap<u32, ObjectId>,
    /// Maps output ObjectId to connector name (from wl_output.name).
    pub connector_names: HashMap<ObjectId, String>,
    /// Next sequential monitor index for newly bound outputs.
    pub next_output_index: u32,
    /// Last broadcasted active workspace (name, id, monitor_index).
    pub last_active: Option<(String, i32, u32)>,
    /// Workspaces that have already been announced via lifecycle events.
    pub broadcasted_workspaces: HashSet<ObjectId>,
    /// Workspaces created since last `Done` event, pending lifecycle broadcast.
    pub pending_new_workspaces: Vec<ObjectId>,
}

impl WaylandState {
    /// Create a new `WaylandState` with the given event senders.
    pub fn new(workspace_sender: mpsc::UnboundedSender<WorkspaceEvent>, monitor_sender: mpsc::UnboundedSender<MonitorEvent>) -> Self {
        Self {
            workspace_sender,
            monitor_sender,
            manager: None,
            workspaces: HashMap::new(),
            groups: HashMap::new(),
            outputs: HashMap::new(),
            output_to_index: HashMap::new(),
            global_name_to_output: HashMap::new(),
            connector_names: HashMap::new(),
            next_output_index: 0,
            last_active: None,
            broadcasted_workspaces: HashSet::new(),
            pending_new_workspaces: Vec::new(),
        }
    }

    /// Find the currently active workspace and resolve its monitor index.
    pub fn find_active_workspace(&self) -> Option<(String, i32, u32)> {
        for (_, ws) in &self.workspaces {
            if ws.is_active {
                let monitor_index = ws
                    .group_id
                    .as_ref()
                    .and_then(|gid| self.groups.get(gid))
                    .and_then(|g| g.output_ids.first())
                    .and_then(|oid| self.output_to_index.get(oid))
                    .copied()
                    .unwrap_or(0);

                let id_num = ws.id.parse::<i32>().or_else(|_| ws.name.parse::<i32>()).unwrap_or(-1);
                return Some((ws.name.clone(), id_num, monitor_index));
            }
        }
        None
    }

    /// Find the ObjectId and monitor index for an output by its global name.
    pub fn find_output_by_global_name(&self, global_name: u32) -> Option<(ObjectId, u32)> {
        self.global_name_to_output
            .get(&global_name)
            .and_then(|id| self.output_to_index.get(id).map(|idx| (id.clone(), *idx)))
    }

    /// Process pending state changes after a `Done` event.
    pub fn process_done(&mut self) {
        // Broadcast workspace lifecycle events for newly created workspaces.
        let pending = std::mem::take(&mut self.pending_new_workspaces);
        for ws_id in pending {
            if self.broadcasted_workspaces.contains(&ws_id) {
                continue;
            }
            if let Some(ws) = self.workspaces.get(&ws_id) {
                let monitor_index = ws
                    .group_id
                    .as_ref()
                    .and_then(|gid| self.groups.get(gid))
                    .and_then(|g| g.output_ids.first())
                    .and_then(|oid| self.output_to_index.get(oid))
                    .copied()
                    .unwrap_or(0);

                let id_num = ws.id.parse::<i32>().or_else(|_| ws.name.parse::<i32>()).unwrap_or(-1);
                let event = WorkspaceLifecycleEvent {
                    workspace_name: ws.name.clone().into(),
                    workspace_id: id_num,
                    monitor_index,
                    lifecycle_type: WorkspaceLifecycleType::Created,
                };
                debug!("Wayland workspace created: {:?}", event);
                let _ = self.workspace_sender.send(WorkspaceEvent::WorkspaceLifecycle(event));
                self.broadcasted_workspaces.insert(ws_id);
            }
        }

        let active = self.find_active_workspace();
        if let Some((name, id_num, monitor_index)) = active {
            let changed = self.last_active.as_ref().map_or(true, |prev| prev.0 != name || prev.1 != id_num);

            if changed {
                self.last_active = Some((name.clone(), id_num, monitor_index));
                let event = WorkspaceChangedEvent {
                    workspace_name: name.into(),
                    workspace_id: id_num,
                    monitor_index,
                };
                debug!("Wayland workspace changed: {:?}", event);
                let _ = self.workspace_sender.send(WorkspaceEvent::WorkspaceChanged(event));
            }
        }
    }
}
