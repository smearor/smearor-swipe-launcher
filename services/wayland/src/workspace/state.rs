use smearor_workspace_model::WorkspaceChangedEvent;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::debug;
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1;

/// Internal event sent from the Wayland listener thread to the worker thread.
pub enum WorkspaceEvent {
    /// Active workspace changed.
    WorkspaceChanged(WorkspaceChangedEvent),
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
    /// Channel for sending workspace change events to the worker thread.
    pub sender: mpsc::UnboundedSender<WorkspaceEvent>,
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
    /// Next sequential monitor index for newly bound outputs.
    pub next_output_index: u32,
    /// Last broadcasted active workspace (name, id, monitor_index).
    pub last_active: Option<(String, i32, u32)>,
}

impl WaylandState {
    /// Create a new `WaylandState` with the given event sender.
    pub fn new(sender: mpsc::UnboundedSender<WorkspaceEvent>) -> Self {
        Self {
            sender,
            manager: None,
            workspaces: HashMap::new(),
            groups: HashMap::new(),
            outputs: HashMap::new(),
            output_to_index: HashMap::new(),
            next_output_index: 0,
            last_active: None,
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

    /// Process pending state changes after a `Done` event.
    pub fn process_done(&mut self) {
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
                let _ = self.sender.send(WorkspaceEvent::WorkspaceChanged(event));
            }
        }
    }
}
