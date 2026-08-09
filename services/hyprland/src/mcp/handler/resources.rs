use crate::service::HyprlandService;
use smearor_hyprland_model::ActiveWindowEntry;
use smearor_hyprland_model::GroupEvent;
use smearor_hyprland_model::HyprlandMcpResources;
use smearor_hyprland_model::HyprlandStateResponse;
use smearor_hyprland_model::LayerEvent;
use smearor_hyprland_model::SystemEvent;
use smearor_hyprland_model::WindowEvent;
use smearor_hyprland_model::WorkspaceEvent;
use smearor_model_compositor::MonitorChangedEvent;
use smearor_model_compositor::WorkspaceInfo;
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
                    return InvokeResourceResponse::error(correlation_id, "Hyprland state not yet available");
                };
                let active_window = state.active_window.as_ref().map(|w| ActiveWindowEntry {
                    class: w.window_class.to_string(),
                    title: w.window_title.to_string(),
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
                let Some(state) = shared_state.as_ref().and_then(|s| s.last_state.clone()) else {
                    return InvokeResourceResponse::error(correlation_id, "Hyprland state not yet available");
                };
                match state.active_window.as_ref() {
                    Some(w) => {
                        let entry = ActiveWindowEntry {
                            class: w.window_class.to_string(),
                            title: w.window_title.to_string(),
                            workspace_id: w.workspace_id,
                        };
                        let json = serde_json::to_string(&entry).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::WorkspaceSnapshot => {
                let snapshot = shared_state.as_ref().and_then(|s| s.workspace_snapshot.clone());
                match snapshot {
                    Some(snap) => {
                        let json = serde_json::to_string(&snap).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::error(correlation_id, "Workspace snapshot not yet available. Send a snapshot request first."),
                }
            }
            HyprlandMcpResources::Workspaces => {
                let workspaces: Vec<WorkspaceInfo> = shared_state
                    .as_ref()
                    .and_then(|s| s.workspace_snapshot.as_ref())
                    .map(|snap| snap.workspaces.iter().cloned().collect())
                    .unwrap_or_default();
                let json = serde_json::to_string(&workspaces).unwrap_or_default();
                InvokeResourceResponse::success(correlation_id, &json)
            }
            HyprlandMcpResources::Monitors => {
                let monitor: Option<MonitorChangedEvent> = shared_state.as_ref().and_then(|s| s.latest_monitor_changed.clone());
                match monitor {
                    Some(event) => {
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::WindowStatus => {
                let event: Option<WindowEvent> = shared_state.as_ref().and_then(|s| s.latest_window_event.clone());
                match event {
                    Some(e) => {
                        let json = serde_json::to_string(&e).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::WorkspaceStatus => {
                let event: Option<WorkspaceEvent> = shared_state.as_ref().and_then(|s| s.latest_workspace_event.clone());
                match event {
                    Some(e) => {
                        let json = serde_json::to_string(&e).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::GroupStatus => {
                let event: Option<GroupEvent> = shared_state.as_ref().and_then(|s| s.latest_group_event.clone());
                match event {
                    Some(e) => {
                        let json = serde_json::to_string(&e).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::LayerStatus => {
                let event: Option<LayerEvent> = shared_state.as_ref().and_then(|s| s.latest_layer_event.clone());
                match event {
                    Some(e) => {
                        let json = serde_json::to_string(&e).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                }
            }
            HyprlandMcpResources::SystemStatus => {
                let event: Option<SystemEvent> = shared_state.as_ref().and_then(|s| s.latest_system_event.clone());
                match event {
                    Some(e) => {
                        let json = serde_json::to_string(&e).unwrap_or_default();
                        InvokeResourceResponse::success(correlation_id, &json)
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
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
        assert_eq!(response.contents.to_string(), "[]");
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
    }

    #[test]
    fn all_status_resources_return_null_when_empty() {
        let service = test_service();
        let status_resources = [
            HyprlandMcpResources::Monitors,
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
}
