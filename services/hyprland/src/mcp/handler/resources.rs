use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_hyprland_model::ActiveWindowEntry;
use smearor_hyprland_model::GroupEvent;
use smearor_hyprland_model::GroupStatusEvent;
use smearor_hyprland_model::GroupStatusResponse;
use smearor_hyprland_model::HyprlandMcpResources;
use smearor_hyprland_model::HyprlandStateResponse;
use smearor_hyprland_model::LayerEvent;
use smearor_hyprland_model::LayerStatusEvent;
use smearor_hyprland_model::LayerStatusResponse;
use smearor_hyprland_model::MonitorsResponse;
use smearor_hyprland_model::SystemEvent;
use smearor_hyprland_model::SystemStatusEvent;
use smearor_hyprland_model::SystemStatusResponse;
use smearor_hyprland_model::WindowEvent;
use smearor_hyprland_model::WindowStatusEvent;
use smearor_hyprland_model::WindowStatusResponse;
use smearor_hyprland_model::WindowsResponse;
use smearor_hyprland_model::WorkspaceEntry;
use smearor_hyprland_model::WorkspaceEvent;
use smearor_hyprland_model::WorkspaceSnapshotResponse;
use smearor_hyprland_model::WorkspaceStatusEvent;
use smearor_hyprland_model::WorkspaceStatusResponse;
use smearor_hyprland_model::WorkspacesResponse;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<HyprlandMcpResources> for HyprlandService {
    fn get_response(&self, request: &ResourceRequest<HyprlandMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let shared_state = self.shared_state.lock().ok();

        match request.resource {
            HyprlandMcpResources::State => {
                let Some(state) = shared_state.as_ref().and_then(|s| s.last_state.clone()) else {
                    let _ = self.command_sender.send(HyprlandCommand::StateRequest);
                    return InvokeResourceResponse::error(
                        correlation_id,
                        "Hyprland state not yet available. A state request has been triggered; please retry shortly.",
                    );
                };
                let active_window = state.active_window.as_ref().map(|w| ActiveWindowEntry {
                    class: w.window_class.to_string(),
                    title: w.window_title.to_string(),
                    address: w.window_address.to_string(),
                    workspace_id: w.workspace_id,
                });
                let response = HyprlandStateResponse {
                    active_window,
                    is_fullscreen: state.is_fullscreen,
                    keyboard_layout: state.keyboard_layout.as_ref().map(|l| l.to_string()).unwrap_or_else(|| "unknown".to_string()),
                    submap: state.sub_map.to_string(),
                };
                let json = serde_json::to_string(&response).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
            HyprlandMcpResources::ActiveWindow => {
                let state = shared_state.as_ref().and_then(|s| s.last_state.clone());
                match state.as_ref().and_then(|st| st.active_window.as_ref()) {
                    Some(w) => {
                        let entry = ActiveWindowEntry {
                            class: w.window_class.to_string(),
                            title: w.window_title.to_string(),
                            address: w.window_address.to_string(),
                            workspace_id: w.workspace_id,
                        };
                        let json = serde_json::to_string(&entry).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => {
                        // Fallback: extract from latest_window_event if available
                        let window_event = shared_state.as_ref().and_then(|s| s.latest_window_event.clone());
                        match window_event.and_then(|e| {
                            e.match_ref(
                                |awc| {
                                    awc.data.as_ref().map(|d| ActiveWindowEntry {
                                        class: d.window_class.to_string(),
                                        title: d.window_title.to_string(),
                                        address: d.window_address.to_string(),
                                        workspace_id: d.workspace_id,
                                    })
                                },
                                |opened| {
                                    Some(ActiveWindowEntry {
                                        class: opened.data.data.window_class.to_string(),
                                        title: opened.data.data.window_title.to_string(),
                                        address: opened.data.data.window_address.to_string(),
                                        workspace_id: opened.data.data.workspace_id,
                                    })
                                },
                                |_| None,
                                |_| None,
                                |_| None,
                                |_| None,
                                |_| None,
                                |_| None,
                            )
                        }) {
                            Some(entry) => {
                                let json = serde_json::to_string(&entry).unwrap_or_default();
                                InvokeResourceResponse::success(correlation_id, &json)
                            }
                            None => InvokeResourceResponse::success(correlation_id, "null"),
                        }
                    }
                }
            }
            HyprlandMcpResources::WorkspaceSnapshot => {
                let snapshot = shared_state.as_ref().and_then(|s| s.workspace_snapshot.clone());
                match snapshot {
                    Some(snap) => {
                        let workspaces: Vec<WorkspaceEntry> = snap
                            .workspaces
                            .iter()
                            .map(|w| WorkspaceEntry {
                                workspace_id: w.workspace_id,
                                workspace_name: w.workspace_name.to_string(),
                                monitor_index: w.monitor_index,
                                is_active: w.is_active,
                            })
                            .collect();
                        let response = WorkspaceSnapshotResponse {
                            workspaces,
                            active_workspace_id: snap.active_workspace_id,
                            active_monitor_index: snap.active_monitor_index,
                        };
                        let json = serde_json::to_string(&response).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => {
                        let _ = self
                            .command_sender
                            .send(HyprlandCommand::SnapshotRequest(WorkspaceSnapshotRequestMessage { monitor_index: 0 }));
                        InvokeResourceResponse::error(
                            correlation_id,
                            "Workspace snapshot not yet available. A snapshot request has been triggered; please retry shortly.",
                        )
                    }
                }
            }
            HyprlandMcpResources::Workspaces => {
                let workspaces: Vec<WorkspaceEntry> = shared_state
                    .as_ref()
                    .and_then(|s| s.workspace_snapshot.as_ref())
                    .map(|snap| {
                        snap.workspaces
                            .iter()
                            .map(|w| WorkspaceEntry {
                                workspace_id: w.workspace_id,
                                workspace_name: w.workspace_name.to_string(),
                                monitor_index: w.monitor_index,
                                is_active: w.is_active,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let response = WorkspacesResponse { workspaces };
                let json = serde_json::to_string(&response).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
            HyprlandMcpResources::Monitors => {
                let monitors = shared_state.as_ref().and_then(|s| s.last_monitors.clone());
                match monitors {
                    Some(entries) if !entries.is_empty() => {
                        let response = MonitorsResponse { monitors: entries };
                        let json = serde_json::to_string(&response).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    _ => {
                        let _ = self.command_sender.send(HyprlandCommand::MonitorsRequest);
                        InvokeResourceResponse::error(
                            correlation_id,
                            "Monitors list not yet available. A monitors request has been triggered; please retry shortly.",
                        )
                    }
                }
            }
            HyprlandMcpResources::Version => {
                let version = shared_state.as_ref().and_then(|s| s.last_version.clone());
                match version {
                    Some(v) => {
                        let json = serde_json::to_string(&v).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => {
                        let _ = self.command_sender.send(HyprlandCommand::VersionRequest);
                        InvokeResourceResponse::error(
                            correlation_id,
                            "Version info not yet available. A version request has been triggered; please retry shortly.",
                        )
                    }
                }
            }
            HyprlandMcpResources::WindowStatus => {
                let event: Option<WindowEvent> = shared_state.as_ref().and_then(|s| s.latest_window_event.clone());
                match event {
                    Some(e) => {
                        let entry = window_event_to_dto(&e);
                        let response = WindowStatusResponse { events: vec![entry] };
                        let json = serde_json::to_string(&response).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::WorkspaceStatus => {
                let event: Option<WorkspaceEvent> = shared_state.as_ref().and_then(|s| s.latest_workspace_event.clone());
                match event {
                    Some(e) => {
                        let entry = workspace_event_to_dto(&e);
                        let response = WorkspaceStatusResponse { events: vec![entry] };
                        let json = serde_json::to_string(&response).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::GroupStatus => {
                let event: Option<GroupEvent> = shared_state.as_ref().and_then(|s| s.latest_group_event.clone());
                match event {
                    Some(e) => {
                        let entry = group_event_to_dto(&e);
                        let response = GroupStatusResponse { events: vec![entry] };
                        let json = serde_json::to_string(&response).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::LayerStatus => {
                let event: Option<LayerEvent> = shared_state.as_ref().and_then(|s| s.latest_layer_event.clone());
                match event {
                    Some(e) => {
                        let entry = layer_event_to_dto(&e);
                        let response = LayerStatusResponse { events: vec![entry] };
                        let json = serde_json::to_string(&response).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::SystemStatus => {
                let event: Option<SystemEvent> = shared_state.as_ref().and_then(|s| s.latest_system_event.clone());
                match event {
                    Some(e) => {
                        let entry = system_event_to_dto(&e);
                        let response = SystemStatusResponse { events: vec![entry] };
                        let json = serde_json::to_string(&response).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::Windows => {
                let windows = shared_state.as_ref().and_then(|s| s.last_windows.clone());
                match windows {
                    Some(wins) => {
                        let response = WindowsResponse { windows: wins };
                        let json = serde_json::to_string(&response).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => {
                        let _ = self.command_sender.send(HyprlandCommand::WindowsRequest);
                        InvokeResourceResponse::error(
                            correlation_id,
                            "Windows list not yet available. A windows request has been triggered; please retry shortly.",
                        )
                    }
                }
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}

fn window_event_to_dto(event: &WindowEvent) -> WindowStatusEvent {
    event.match_ref(
        |awc| {
            let d = awc.data.as_ref();
            WindowStatusEvent {
                event_type: "active_changed".to_string(),
                class: d.map(|d| d.window_class.to_string()),
                title: d.map(|d| d.window_title.to_string()),
                workspace_id: d.map(|d| d.workspace_id),
            }
        },
        |opened| WindowStatusEvent {
            event_type: "opened".to_string(),
            class: Some(opened.data.data.window_class.to_string()),
            title: Some(opened.data.data.window_title.to_string()),
            workspace_id: Some(opened.data.data.workspace_id),
        },
        |_closed| WindowStatusEvent {
            event_type: "closed".to_string(),
            class: None,
            title: None,
            workspace_id: None,
        },
        |moved| WindowStatusEvent {
            event_type: "moved".to_string(),
            class: None,
            title: None,
            workspace_id: Some(moved.data.workspace_id),
        },
        |_float_state| WindowStatusEvent {
            event_type: "float_state_changed".to_string(),
            class: None,
            title: None,
            workspace_id: None,
        },
        |_urgent_state| WindowStatusEvent {
            event_type: "urgent_state_changed".to_string(),
            class: None,
            title: None,
            workspace_id: None,
        },
        |title_changed| WindowStatusEvent {
            event_type: "title_changed".to_string(),
            class: None,
            title: Some(title_changed.data.window_title.to_string()),
            workspace_id: None,
        },
        |_pinned| WindowStatusEvent {
            event_type: "pinned".to_string(),
            class: None,
            title: None,
            workspace_id: None,
        },
    )
}

fn workspace_event_to_dto(event: &WorkspaceEvent) -> WorkspaceStatusEvent {
    event.match_ref(
        |_| WorkspaceStatusEvent {
            event_type: "fullscreen_state_changed".to_string(),
            workspace_id: None,
            workspace_name: None,
        },
        |renamed| WorkspaceStatusEvent {
            event_type: "renamed".to_string(),
            workspace_id: Some(renamed.data.workspace_id),
            workspace_name: Some(renamed.data.workspace_name.to_string()),
        },
        |_| WorkspaceStatusEvent {
            event_type: "special_removed".to_string(),
            workspace_id: None,
            workspace_name: None,
        },
        |changed_special| WorkspaceStatusEvent {
            event_type: "changed_special".to_string(),
            workspace_id: None,
            workspace_name: Some(changed_special.data.special_workspace_name.to_string()),
        },
        |sub_map_changed| WorkspaceStatusEvent {
            event_type: "sub_map_changed".to_string(),
            workspace_id: None,
            workspace_name: Some(sub_map_changed.sub_map.to_string()),
        },
    )
}

fn group_event_to_dto(event: &GroupEvent) -> GroupStatusEvent {
    event.match_ref(
        |_| GroupStatusEvent {
            event_type: "toggled".to_string(),
        },
        |_| GroupStatusEvent {
            event_type: "moved_into".to_string(),
        },
        |_| GroupStatusEvent {
            event_type: "moved_out".to_string(),
        },
        |_| GroupStatusEvent {
            event_type: "ignore_lock_changed".to_string(),
        },
        |_| GroupStatusEvent {
            event_type: "lock_changed".to_string(),
        },
    )
}

fn layer_event_to_dto(event: &LayerEvent) -> LayerStatusEvent {
    event.match_ref(
        |opened| LayerStatusEvent {
            event_type: "opened".to_string(),
            namespace: Some(opened.layer_name.to_string()),
        },
        |closed| LayerStatusEvent {
            event_type: "closed".to_string(),
            namespace: Some(closed.layer_name.to_string()),
        },
    )
}

fn system_event_to_dto(event: &SystemEvent) -> SystemStatusEvent {
    event.match_ref(
        |_| SystemStatusEvent {
            event_type: "keyboard_layout_changed".to_string(),
        },
        |_| SystemStatusEvent {
            event_type: "screencast".to_string(),
        },
        |_| SystemStatusEvent {
            event_type: "config_reloaded".to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::config::HyprlandServiceConfig;
    use crate::service::HyprlandCommand;
    use crate::service::HyprlandService;
    use crate::service::HyprlandSharedState;
    use smearor_hyprland_model::HyprlandMcpResources;
    use smearor_hyprland_shared::event::window_event_data::HyprlandWindowEventData;
    use smearor_hyprland_status::HyprlandStateMessage;
    use smearor_model_compositor::WorkspaceInfo;
    use smearor_model_compositor::WorkspaceSnapshotMessage;
    use smearor_model_mcp::resources::handler::McpResourceHandler;
    use smearor_model_mcp::resources::handler::ResourceRequest;
    use smearor_swipe_launcher_plugin_api::PluginMeta;
    use std::sync::Arc;
    use std::sync::Mutex;

    fn stabby_vec<T: stabby::IStable>(items: Vec<T>) -> stabby::vec::Vec<T> {
        items.into_iter().collect()
    }

    fn test_service() -> HyprlandService {
        let (command_sender, _command_receiver) = tokio::sync::mpsc::unbounded_channel::<HyprlandCommand>();
        HyprlandService {
            meta: PluginMeta::new("test-hyprland".to_string(), "Test Hyprland".to_string(), None),
            core_context: None,
            command_sender,
            config: Arc::new(HyprlandServiceConfig::default()),
            shared_state: Arc::new(Mutex::new(HyprlandSharedState::default())),
        }
    }

    fn service_with_state(state: HyprlandStateMessage) -> HyprlandService {
        let service = test_service();
        service.shared_state.lock().unwrap().last_state = Some(state);
        service
    }

    fn service_with_snapshot(snapshot: WorkspaceSnapshotMessage) -> HyprlandService {
        let service = test_service();
        service.shared_state.lock().unwrap().workspace_snapshot = Some(snapshot);
        service
    }

    #[test]
    fn state_resource_returns_error_when_no_state() {
        let service = test_service();
        let request = ResourceRequest {
            resource: HyprlandMcpResources::State,
            correlation_id: "corr-1",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert_eq!(response.correlation_id.to_string(), "corr-1");
        assert!(response.contents.is_empty(), "error response should have empty contents");
    }

    #[test]
    fn state_resource_returns_json_when_state_present() {
        let state = HyprlandStateMessage {
            active_window: Some(HyprlandWindowEventData {
                window_class: "firefox".into(),
                window_title: "Mozilla Firefox".into(),
                window_address: "0x123".into(),
                workspace_id: 1,
            })
            .into(),
            is_fullscreen: true,
            keyboard_layout: Some("de".into()).into(),
            sub_map: "".into(),
            ignore_group_lock: false,
            groups_locked: false,
        };
        let service = service_with_state(state);
        let request = ResourceRequest {
            resource: HyprlandMcpResources::State,
            correlation_id: "corr-2",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert_eq!(response.correlation_id.to_string(), "corr-2");
        assert!(!response.contents.is_empty(), "success response should have contents");
        assert!(response.contents.contains("firefox"), "response should contain window class");
        assert!(response.contents.contains("true"), "response should contain fullscreen state");
    }

    #[test]
    fn active_window_resource_returns_null_when_no_window() {
        let state = HyprlandStateMessage::default();
        let service = service_with_state(state);
        let request = ResourceRequest {
            resource: HyprlandMcpResources::ActiveWindow,
            correlation_id: "corr-3",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert_eq!(response.contents.to_string(), "null");
    }

    #[test]
    fn active_window_resource_returns_json_when_window_present() {
        let state = HyprlandStateMessage {
            active_window: Some(HyprlandWindowEventData {
                window_class: "kitty".into(),
                window_title: "kitty".into(),
                window_address: "0x456".into(),
                workspace_id: 2,
            })
            .into(),
            ..Default::default()
        };
        let service = service_with_state(state);
        let request = ResourceRequest {
            resource: HyprlandMcpResources::ActiveWindow,
            correlation_id: "corr-4",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.contains("kitty"), "response should contain window class");
    }

    #[test]
    fn workspace_snapshot_returns_error_when_empty() {
        let service = test_service();
        let request = ResourceRequest {
            resource: HyprlandMcpResources::WorkspaceSnapshot,
            correlation_id: "corr-5",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.is_empty(), "error response should have empty contents");
    }

    #[test]
    fn workspace_snapshot_returns_json_when_present() {
        let snapshot = WorkspaceSnapshotMessage {
            workspaces: stabby_vec(vec![WorkspaceInfo {
                workspace_id: 1,
                workspace_name: "web".into(),
                monitor_index: 0,
                is_active: true,
            }]),
            active_workspace_id: 1,
            active_monitor_index: 0,
        };
        let service = service_with_snapshot(snapshot);
        let request = ResourceRequest {
            resource: HyprlandMcpResources::WorkspaceSnapshot,
            correlation_id: "corr-6",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.contains("web"), "response should contain workspace name");
        assert!(response.contents.contains("\"active_workspace_id\":1"), "response should contain active workspace id");
        assert!(response.contents.contains("\"workspaces\""), "response should use DTO format with workspaces field");
    }

    #[test]
    fn workspaces_resource_returns_empty_array_when_no_snapshot() {
        let service = test_service();
        let request = ResourceRequest {
            resource: HyprlandMcpResources::Workspaces,
            correlation_id: "corr-7",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.contains("\"workspaces\":[]"), "response should use DTO format with empty workspaces array");
    }

    #[test]
    fn workspaces_resource_returns_array_when_snapshot_present() {
        let snapshot = WorkspaceSnapshotMessage {
            workspaces: stabby_vec(vec![
                WorkspaceInfo {
                    workspace_id: 1,
                    workspace_name: "web".into(),
                    monitor_index: 0,
                    is_active: true,
                },
                WorkspaceInfo {
                    workspace_id: 2,
                    workspace_name: "dev".into(),
                    monitor_index: 0,
                    is_active: false,
                },
            ]),
            active_workspace_id: 1,
            active_monitor_index: 0,
        };
        let service = service_with_snapshot(snapshot);
        let request = ResourceRequest {
            resource: HyprlandMcpResources::Workspaces,
            correlation_id: "corr-8",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.contains("web"));
        assert!(response.contents.contains("dev"));
        assert!(response.contents.contains("\"workspaces\""), "response should use DTO format with workspaces field");
    }

    #[test]
    fn all_status_resources_return_null_when_empty() {
        let service = test_service();
        let status_resources = [
            HyprlandMcpResources::WindowStatus,
            HyprlandMcpResources::WorkspaceStatus,
            HyprlandMcpResources::GroupStatus,
            HyprlandMcpResources::LayerStatus,
            HyprlandMcpResources::SystemStatus,
        ];
        for (i, resource) in status_resources.iter().enumerate() {
            let request = ResourceRequest {
                resource: *resource,
                correlation_id: "corr-status",
                sender_id: "test",
            };
            let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
            assert_eq!(
                response.contents.to_string(),
                "null",
                "resource {:?} should return null when empty (iteration {})",
                resource,
                i
            );
        }
    }

    #[test]
    fn windows_resource_returns_error_when_no_windows() {
        let service = test_service();
        let request = ResourceRequest {
            resource: HyprlandMcpResources::Windows,
            correlation_id: "corr-windows-1",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.is_empty(), "error response should have empty contents");
    }

    #[test]
    fn windows_resource_returns_json_when_windows_present() {
        use smearor_hyprland_model::WindowEntry;
        let service = test_service();
        service.shared_state.lock().unwrap().last_windows = Some(vec![
            WindowEntry {
                class: "firefox".to_string(),
                title: "Mozilla Firefox".to_string(),
                address: "0x123".to_string(),
                workspace_id: 1,
                monitor: Some(0),
                floating: false,
                fullscreen_mode: "fullscreen".to_string(),
                pinned: false,
                mapped: true,
                pid: 1234,
                is_active: true,
            },
            WindowEntry {
                class: "kitty".to_string(),
                title: "kitty".to_string(),
                address: "0x456".to_string(),
                workspace_id: 2,
                monitor: Some(0),
                floating: false,
                fullscreen_mode: "none".to_string(),
                pinned: false,
                mapped: true,
                pid: 5678,
                is_active: false,
            },
        ]);
        let request = ResourceRequest {
            resource: HyprlandMcpResources::Windows,
            correlation_id: "corr-windows-2",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.contains("firefox"), "response should contain firefox");
        assert!(response.contents.contains("kitty"), "response should contain kitty");
        assert!(response.contents.contains("\"windows\""), "response should use DTO format with windows field");
        assert!(response.contents.contains("\"is_active\":true"), "response should contain is_active field");
    }

    #[test]
    fn monitors_resource_returns_error_when_no_monitors() {
        let service = test_service();
        let request = ResourceRequest {
            resource: HyprlandMcpResources::Monitors,
            correlation_id: "corr-monitors-1",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.is_empty(), "error response should have empty contents");
    }

    #[test]
    fn monitors_resource_returns_json_when_monitors_present() {
        use smearor_hyprland_model::MonitorEntry;
        let service = test_service();
        service.shared_state.lock().unwrap().last_monitors = Some(vec![
            MonitorEntry {
                monitor_index: 0,
                connector_name: "eDP-1".to_string(),
                width: 1920,
                height: 1080,
                refresh_rate: 60.0,
                x: 0,
                y: 0,
                active_workspace_id: 1,
                active_workspace_name: "web".to_string(),
                scale: 1.0,
                transform: "normal".to_string(),
                focused: true,
                dpms_status: true,
                vrr: false,
                disabled: false,
            },
            MonitorEntry {
                monitor_index: 1,
                connector_name: "HDMI-A-1".to_string(),
                width: 2560,
                height: 1440,
                refresh_rate: 144.0,
                x: 1920,
                y: 0,
                active_workspace_id: 2,
                active_workspace_name: "dev".to_string(),
                scale: 1.25,
                transform: "normal".to_string(),
                focused: false,
                dpms_status: true,
                vrr: true,
                disabled: false,
            },
        ]);
        let request = ResourceRequest {
            resource: HyprlandMcpResources::Monitors,
            correlation_id: "corr-monitors-2",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.contains("eDP-1"), "response should contain connector name");
        assert!(response.contents.contains("HDMI-A-1"), "response should contain second monitor");
        assert!(response.contents.contains("\"monitors\""), "response should use DTO format with monitors field");
        assert!(response.contents.contains("\"focused\":true"), "response should contain focused field");
        assert!(response.contents.contains("\"dpms_status\":true"), "response should contain dpms_status field");
        assert!(response.contents.contains("\"vrr\":true"), "response should contain vrr field");
        assert!(response.contents.contains("\"active_workspace_id\":1"), "response should contain active_workspace_id field");
    }

    #[test]
    fn version_resource_returns_error_when_no_version() {
        let service = test_service();
        let request = ResourceRequest {
            resource: HyprlandMcpResources::Version,
            correlation_id: "corr-version-1",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.is_empty(), "error response should have empty contents");
    }

    #[test]
    fn version_resource_returns_json_when_version_present() {
        use smearor_hyprland_model::VersionResponse;
        let service = test_service();
        service.shared_state.lock().unwrap().last_version = Some(VersionResponse {
            tag: "0.49.0".to_string(),
            branch: "main".to_string(),
            commit: "abc123".to_string(),
            dirty: false,
            commit_message: "feat: something".to_string(),
            commit_date: "2024-01-01".to_string(),
            commits: "1234".to_string(),
            build_aquamarine: "0.1.0".to_string(),
            flags: vec!["debug".to_string()],
        });
        let request = ResourceRequest {
            resource: HyprlandMcpResources::Version,
            correlation_id: "corr-version-2",
            sender_id: "test",
        };
        let response = <HyprlandService as McpResourceHandler<HyprlandMcpResources>>::get_response(&service, &request);
        assert!(response.contents.contains("0.49.0"), "response should contain tag");
        assert!(response.contents.contains("main"), "response should contain branch");
        assert!(response.contents.contains("abc123"), "response should contain commit");
        assert!(response.contents.contains("\"flags\""), "response should contain flags array");
        assert!(response.contents.contains("\"commit_date\""), "response should contain commit_date field");
        assert!(response.contents.contains("\"build_aquamarine\""), "response should contain build_aquamarine field");
    }
}
