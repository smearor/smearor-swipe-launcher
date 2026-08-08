use crate::mcp::McpResponseTracker;
use crate::mcp::plugin_invoke::invoke_plugin_resource_sender;
use smearor_mcp_server::CommandResponseWrapper;
use smearor_mcp_server::InvokePluginResourceParams;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use tokio::sync::mpsc::UnboundedSender;

/// Handle a plugin resource read by sending the request via the broker
/// and awaiting the response with a 10-second timeout.
pub(crate) async fn handle_plugin_resource(
    broker_sender: &UnboundedSender<FfiEnvelope>,
    response_tracker: &McpResponseTracker,
    command: CommandResponseWrapper<InvokePluginResourceParams>,
) {
    let receiver = response_tracker.register(command.params.correlation_id.clone());
    let send_result = invoke_plugin_resource_sender(broker_sender, &command.params.uri, &command.params.correlation_id);
    if send_result.is_ok() {
        let response_result = match tokio::time::timeout(tokio::time::Duration::from_secs(10), receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Plugin resource read dropped".to_string()),
            Err(_) => Err("Plugin resource read timed out".to_string()),
        };
        let _ = command.response.send(response_result);
    } else {
        let _ = command.response.send(send_result.map(|_| String::new()).map_err(|e| e.to_string()));
    }
}
