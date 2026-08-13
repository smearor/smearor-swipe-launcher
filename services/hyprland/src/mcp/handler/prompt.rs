use crate::service::HyprlandService;
use smearor_hyprland_model::HyprlandMcpPrompts;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_model_mcp::render_template;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl HyprlandService {
    fn active_window_class(&self) -> String {
        self.status_snapshot()
            .and_then(|s| s.active_window.as_ref().map(|w| w.window_class.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn active_workspace_id(&self) -> String {
        self.shared_state
            .lock()
            .ok()
            .and_then(|s| s.workspace_snapshot.as_ref().map(|snap| snap.active_workspace_id.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn workspace_count(&self) -> String {
        self.shared_state
            .lock()
            .ok()
            .and_then(|s| s.workspace_snapshot.as_ref().map(|snap| snap.workspaces.len().to_string()))
            .unwrap_or_else(|| "0".to_string())
    }

    fn is_fullscreen(&self) -> String {
        self.status_snapshot()
            .map(|s| s.is_fullscreen.to_string())
            .unwrap_or_else(|| "false".to_string())
    }

    fn keyboard_layout(&self) -> String {
        self.status_snapshot()
            .and_then(|s| s.keyboard_layout.as_ref().map(|l| l.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for HyprlandService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("hyprland: InvokePromptMessage name={} sender_id={}", prompt_name, sender_id);
        let prompt = match HyprlandMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                self.send_response(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)), sender_id);
                return;
            }
        };

        let active_window_class = self.active_window_class();
        let active_workspace_id = self.active_workspace_id();
        let workspace_count = self.workspace_count();
        let is_fullscreen = self.is_fullscreen();
        let keyboard_layout = self.keyboard_layout();

        let response = match prompt {
            HyprlandMcpPrompts::HyprlandOverview => {
                let content = render_template(
                    include_str!("../../../data/prompts/overview.md"),
                    &[
                        ("active_window_class", &active_window_class),
                        ("active_workspace_id", &active_workspace_id),
                        ("is_fullscreen", &is_fullscreen),
                        ("keyboard_layout", &keyboard_layout),
                    ],
                );
                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
            HyprlandMcpPrompts::HyprlandQuickReference => {
                let content = render_template(
                    include_str!("../../../data/prompts/quick_reference.md"),
                    &[
                        ("active_window_class", &active_window_class),
                        ("active_workspace_id", &active_workspace_id),
                        ("is_fullscreen", &is_fullscreen),
                        ("keyboard_layout", &keyboard_layout),
                    ],
                );
                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
            HyprlandMcpPrompts::HyprlandWindowGuide => {
                let content = render_template(
                    include_str!("../../../data/prompts/window_guide.md"),
                    &[
                        ("active_window_class", &active_window_class),
                        ("active_workspace_id", &active_workspace_id),
                        ("is_fullscreen", &is_fullscreen),
                    ],
                );
                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
            HyprlandMcpPrompts::HyprlandWorkspaceGuide => {
                let content = render_template(
                    include_str!("../../../data/prompts/workspace_guide.md"),
                    &[("active_workspace_id", &active_workspace_id), ("workspace_count", &workspace_count)],
                );
                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };
        self.send_response(response, sender_id);
    }
}

#[cfg(test)]
mod tests {
    use crate::config::HyprlandServiceConfig;
    use crate::service::HyprlandCommand;
    use crate::service::HyprlandService;
    use crate::service::HyprlandSharedState;
    use smearor_hyprland_shared::event::window_event_data::HyprlandWindowEventData;
    use smearor_hyprland_status::HyprlandStateMessage;
    use smearor_model_compositor::WorkspaceInfo;
    use smearor_model_compositor::WorkspaceSnapshotMessage;
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

    #[test]
    fn active_window_class_returns_unknown_when_no_state() {
        let service = test_service();
        assert_eq!(service.active_window_class(), "unknown");
    }

    #[test]
    fn active_window_class_returns_class_when_state_present() {
        let service = test_service();
        service.shared_state.lock().unwrap().last_state = Some(HyprlandStateMessage {
            active_window: Some(HyprlandWindowEventData {
                window_class: "firefox".into(),
                window_title: "Firefox".into(),
                window_address: "0x1".into(),
                workspace_id: stabby::option::Option::Some(1),
            })
            .into(),
            ..Default::default()
        });
        assert_eq!(service.active_window_class(), "firefox");
    }

    #[test]
    fn active_workspace_id_returns_unknown_when_no_snapshot() {
        let service = test_service();
        assert_eq!(service.active_workspace_id(), "unknown");
    }

    #[test]
    fn active_workspace_id_returns_id_when_snapshot_present() {
        let service = test_service();
        service.shared_state.lock().unwrap().workspace_snapshot = Some(WorkspaceSnapshotMessage {
            workspaces: stabby_vec(vec![WorkspaceInfo::default()]),
            active_workspace_id: 5,
            active_monitor_index: 0,
        });
        assert_eq!(service.active_workspace_id(), "5");
    }

    #[test]
    fn workspace_count_returns_zero_when_no_snapshot() {
        let service = test_service();
        assert_eq!(service.workspace_count(), "0");
    }

    #[test]
    fn workspace_count_returns_count_when_snapshot_present() {
        let service = test_service();
        service.shared_state.lock().unwrap().workspace_snapshot = Some(WorkspaceSnapshotMessage {
            workspaces: stabby_vec(vec![
                WorkspaceInfo {
                    workspace_id: 1,
                    ..Default::default()
                },
                WorkspaceInfo {
                    workspace_id: 2,
                    ..Default::default()
                },
                WorkspaceInfo {
                    workspace_id: 3,
                    ..Default::default()
                },
            ]),
            active_workspace_id: 1,
            active_monitor_index: 0,
        });
        assert_eq!(service.workspace_count(), "3");
    }

    #[test]
    fn is_fullscreen_returns_false_when_no_state() {
        let service = test_service();
        assert_eq!(service.is_fullscreen(), "false");
    }

    #[test]
    fn is_fullscreen_returns_true_when_state_present() {
        let service = test_service();
        service.shared_state.lock().unwrap().last_state = Some(HyprlandStateMessage {
            is_fullscreen: true,
            ..Default::default()
        });
        assert_eq!(service.is_fullscreen(), "true");
    }

    #[test]
    fn keyboard_layout_returns_unknown_when_no_state() {
        let service = test_service();
        assert_eq!(service.keyboard_layout(), "unknown");
    }

    #[test]
    fn keyboard_layout_returns_layout_when_state_present() {
        let service = test_service();
        service.shared_state.lock().unwrap().last_state = Some(HyprlandStateMessage {
            keyboard_layout: Some("us".into()).into(),
            ..Default::default()
        });
        assert_eq!(service.keyboard_layout(), "us");
    }
}
