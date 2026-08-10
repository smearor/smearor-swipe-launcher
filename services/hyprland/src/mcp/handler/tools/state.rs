use crate::service::HyprlandCommand;
use crate::service::HyprlandService;
use smearor_model_compositor::WorkspaceSnapshotRequestMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;

impl HyprlandService {
    pub(crate) fn handle_refresh_state_tool(&self, correlation_id: &str, broadcaster: &MessageBroadcasterInner) {
        let _ = self.command_sender.send(HyprlandCommand::StateRequest);
        let _ = self
            .command_sender
            .send(HyprlandCommand::SnapshotRequest(WorkspaceSnapshotRequestMessage { monitor_index: 0 }));
        let response = InvokeToolResponse::success(correlation_id, "State and workspace snapshot refresh requested");
        broadcaster.broadcast_message_to_topic(response);
    }
}
