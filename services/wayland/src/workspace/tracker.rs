use crate::monitor::MonitorEvent;
use crate::workspace::WorkspaceEvent;
use crate::workspace::state::GroupInfo;
use crate::workspace::state::WaylandState;
use crate::workspace::state::WorkspaceInfo;
use smearor_model_compositor::MonitorChangeType;
use smearor_model_compositor::MonitorChangedEvent;
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
                    state.global_name_to_output.insert(name, id.clone());
                    state.outputs.insert(id, output);
                    debug!("Bound wl_output at monitor index {index} (global {name})");

                    // Broadcast monitor connected event.
                    let event = MonitorChangedEvent {
                        monitor_index: index,
                        connector_name: String::new().into(),
                        change_type: MonitorChangeType::Connected,
                    };
                    debug!("Wayland monitor connected: index={index}");
                    let _ = state.monitor_sender.send(MonitorEvent::Changed(event));
                }
            }
            RegistryEvent::GlobalRemove { name } => {
                debug!("Wayland global removed: {name}");
                if let Some((id, index)) = state.find_output_by_global_name(name) {
                    let connector_name = state.connector_names.get(&id).cloned().unwrap_or_default();
                    let event = MonitorChangedEvent {
                        monitor_index: index,
                        connector_name: connector_name.into(),
                        change_type: MonitorChangeType::Disconnected,
                    };
                    debug!("Wayland monitor disconnected: index={index}");
                    let _ = state.monitor_sender.send(MonitorEvent::Changed(event));
                    state.outputs.remove(&id);
                    state.output_to_index.remove(&id);
                    state.connector_names.remove(&id);
                    state.global_name_to_output.remove(&name);
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for WaylandState {
    fn event(state: &mut WaylandState, proxy: &WlOutput, event: OutputEvent, _: &(), _: &Connection, _: &QueueHandle<WaylandState>) {
        let id = proxy.id();
        match event {
            OutputEvent::Name { name } => {
                debug!("wl_output {:?} connector name: {name}", id);
                state.connector_names.insert(id, name);
            }
            _ => {}
        }
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
                    id.clone(),
                    WorkspaceInfo {
                        handle: workspace,
                        name: String::new(),
                        id: String::new(),
                        is_active: false,
                        group_id: None,
                    },
                );
                state.pending_new_workspaces.push(id);
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
                if let Some(ws) = state.workspaces.get(&ws_id) {
                    let monitor_index = ws
                        .group_id
                        .as_ref()
                        .and_then(|gid| state.groups.get(gid))
                        .and_then(|g| g.output_ids.first())
                        .and_then(|oid| state.output_to_index.get(oid))
                        .copied()
                        .unwrap_or(0);

                    let id_num = ws.id.parse::<i32>().or_else(|_| ws.name.parse::<i32>()).unwrap_or(-1);
                    let event = WorkspaceLifecycleEvent {
                        workspace_name: ws.name.clone().into(),
                        workspace_id: id_num,
                        monitor_index,
                        lifecycle_type: WorkspaceLifecycleType::Destroyed,
                    };
                    debug!("Wayland workspace destroyed: {:?}", event);
                    let _ = state.workspace_sender.send(WorkspaceEvent::WorkspaceLifecycle(event));
                }
                state.broadcasted_workspaces.remove(&ws_id);
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
pub fn run_workspace_event_loop(workspace_sender: mpsc::UnboundedSender<WorkspaceEvent>, monitor_sender: mpsc::UnboundedSender<MonitorEvent>) {
    loop {
        match Connection::connect_to_env() {
            Ok(conn) => {
                let display = conn.display();
                let mut event_queue = conn.new_event_queue::<WaylandState>();
                let qh = event_queue.handle();

                let _registry = display.get_registry(&qh, ());

                let mut state = WaylandState::new(workspace_sender.clone(), monitor_sender.clone());

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

/// Spawn the workspace event worker thread that broadcasts workspace events
/// to the launcher core.
pub fn spawn_workspace_worker(mut event_receiver: mpsc::UnboundedReceiver<WorkspaceEvent>, core_context: Option<FfiCoreContext>, meta: PluginMeta) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(error) => {
                tracing::error!("Wayland workspace worker: failed to create runtime: {error}");
                return;
            }
        };

        rt.block_on(async move {
            while let Some(event) = event_receiver.recv().await {
                match event {
                    WorkspaceEvent::WorkspaceChanged(event) => {
                        debug!("Broadcasting workspace changed event: {:?}", event);
                        broadcast_event(&core_context, &meta, event);
                    }
                    WorkspaceEvent::WorkspaceLifecycle(event) => {
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
