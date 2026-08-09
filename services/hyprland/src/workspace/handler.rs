use super::event::WorkspaceEvent;
use crate::event_listener::listener::HyprlandEvent;
use smearor_model_compositor::WorkspaceChangedEvent;
use tokio::sync::mpsc;

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
