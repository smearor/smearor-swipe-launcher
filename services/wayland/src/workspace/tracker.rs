use crate::workspace::WorkspaceEvent;
use crate::workspace::state::GroupInfo;
use crate::workspace::state::WaylandState;
use crate::workspace::state::WorkspaceInfo;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::warn;
use wayland_client::Connection;
use wayland_client::Dispatch;
use wayland_client::Proxy;
use wayland_client::QueueHandle;
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_output::Event as OutputEvent;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_registry::Event as RegistryEvent;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::Event as GroupEvent;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::Event as WorkspaceHandleEvent;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::State as WorkspaceState;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::Event as ManagerEvent;
use wayland_protocols::ext::workspace::v1::client::ext_workspace_manager_v1::ExtWorkspaceManagerV1;

impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(state: &mut WaylandState, registry: &WlRegistry, event: RegistryEvent, _: &(), _: &Connection, qh: &QueueHandle<WaylandState>) {
        match event {
            RegistryEvent::Global { name, interface, version } => {
                if interface == "ext_workspace_manager_v1" {
                    let manager = registry.bind::<ExtWorkspaceManagerV1, (), WaylandState>(name, version.min(1), qh, ());
                    debug!("Bound ext_workspace_manager_v1 (global {name})");
                    state.manager = Some(manager);
                } else if interface == "wl_output" {
                    let output = registry.bind::<WlOutput, (), WaylandState>(name, version.min(4), qh, ());
                    let id = output.id();
                    let index = state.next_output_index;
                    state.next_output_index += 1;
                    state.output_to_index.insert(id.clone(), index);
                    state.outputs.insert(id, output);
                    debug!("Bound wl_output at monitor index {index} (global {name})");
                }
            }
            RegistryEvent::GlobalRemove { name } => {
                debug!("Wayland global removed: {name}");
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for WaylandState {
    fn event(_state: &mut WaylandState, _proxy: &WlOutput, _event: OutputEvent, _: &(), _: &Connection, _: &QueueHandle<WaylandState>) {
        // Output events (geometry, mode, name, etc.) are not needed for
        // workspace tracking. Monitor index is assigned by output bind order.
    }
}

impl Dispatch<ExtWorkspaceManagerV1, ()> for WaylandState {
    fn event(state: &mut WaylandState, _manager: &ExtWorkspaceManagerV1, event: ManagerEvent, _: &(), _: &Connection, _qh: &QueueHandle<WaylandState>) {
        match event {
            ManagerEvent::WorkspaceGroup { workspace_group } => {
                let id: ObjectId = workspace_group.id();
                debug!("Workspace group created: {id}");
                state.groups.insert(
                    id,
                    GroupInfo {
                        handle: workspace_group,
                        output_ids: Vec::new(),
                    },
                );
            }
            ManagerEvent::Workspace { workspace } => {
                let id: ObjectId = workspace.id();
                debug!("Workspace created: {id}");
                state.workspaces.insert(
                    id,
                    WorkspaceInfo {
                        handle: workspace,
                        name: String::new(),
                        id: String::new(),
                        is_active: false,
                        group_id: None,
                    },
                );
            }
            ManagerEvent::Done => {
                state.process_done();
            }
            ManagerEvent::Finished => {
                debug!("Workspace manager finished");
                state.manager = None;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceGroupHandleV1, ()> for WaylandState {
    fn event(state: &mut WaylandState, group: &ExtWorkspaceGroupHandleV1, event: GroupEvent, _: &(), _: &Connection, _: &QueueHandle<WaylandState>) {
        let group_id: ObjectId = group.id();
        match event {
            GroupEvent::OutputEnter { output } => {
                let output_id: ObjectId = output.id();
                if let Some(g) = state.groups.get_mut(&group_id) {
                    if !g.output_ids.contains(&output_id) {
                        g.output_ids.push(output_id.clone());
                    }
                }
                debug!("Output {output_id} entered group {group_id}");
            }
            GroupEvent::OutputLeave { output } => {
                let output_id: ObjectId = output.id();
                if let Some(g) = state.groups.get_mut(&group_id) {
                    g.output_ids.retain(|id| *id != output_id);
                }
                debug!("Output {output_id} left group {group_id}");
            }
            GroupEvent::WorkspaceEnter { workspace } => {
                let ws_id: ObjectId = workspace.id();
                if let Some(ws) = state.workspaces.get_mut(&ws_id) {
                    ws.group_id = Some(group_id.clone());
                }
                debug!("Workspace {ws_id} entered group {group_id}");
            }
            GroupEvent::WorkspaceLeave { workspace } => {
                let ws_id: ObjectId = workspace.id();
                if let Some(ws) = state.workspaces.get_mut(&ws_id) {
                    ws.group_id = None;
                }
                debug!("Workspace {ws_id} left group {group_id}");
            }
            GroupEvent::Removed => {
                debug!("Group {group_id} removed");
                state.groups.remove(&group_id);
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceHandleV1, ()> for WaylandState {
    fn event(state: &mut WaylandState, workspace: &ExtWorkspaceHandleV1, event: WorkspaceHandleEvent, _: &(), _: &Connection, _: &QueueHandle<WaylandState>) {
        let ws_id: ObjectId = workspace.id();
        match event {
            WorkspaceHandleEvent::Id { id } => {
                if let Some(ws) = state.workspaces.get_mut(&ws_id) {
                    ws.id = id;
                }
            }
            WorkspaceHandleEvent::Name { name } => {
                debug!("Workspace {ws_id} name: {name}");
                if let Some(ws) = state.workspaces.get_mut(&ws_id) {
                    ws.name = name;
                }
            }
            WorkspaceHandleEvent::State { state: state_enum } => {
                let is_active = match state_enum {
                    wayland_client::backend::protocol::WEnum::Value(state_val) => state_val.contains(WorkspaceState::Active),
                    wayland_client::backend::protocol::WEnum::Unknown(raw) => (raw & WorkspaceState::Active.bits()) != 0,
                };
                if let Some(ws) = state.workspaces.get_mut(&ws_id) {
                    ws.is_active = is_active;
                }
                debug!("Workspace {ws_id} state: active={is_active}");
            }
            WorkspaceHandleEvent::Removed => {
                debug!("Workspace {ws_id} removed");
                state.workspaces.remove(&ws_id);
            }
            _ => {}
        }
    }
}

/// Run the Wayland event loop with reconnection support.
///
/// Connects to the Wayland display, binds the `ext_workspace_manager_v1`
/// global, and dispatches events. If the connection is lost, retries
/// after 5 seconds.
pub fn run_workspace_event_loop(sender: mpsc::UnboundedSender<WorkspaceEvent>) {
    loop {
        match Connection::connect_to_env() {
            Ok(conn) => {
                let display = conn.display();
                let mut event_queue = conn.new_event_queue::<WaylandState>();
                let qh = event_queue.handle();

                let _registry = display.get_registry(&qh, ());

                let mut state = WaylandState::new(sender.clone());

                debug!("Wayland event loop started, dispatching events");

                loop {
                    match event_queue.blocking_dispatch(&mut state) {
                        Ok(_) => {}
                        Err(error) => {
                            error!("Wayland event loop error: {error}");
                            break;
                        }
                    }
                }

                debug!("Wayland event loop exited, reconnecting in 5s");
            }
            Err(error) => {
                warn!("Failed to connect to Wayland display: {error}, retrying in 5s");
            }
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}
