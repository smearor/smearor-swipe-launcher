use crate::mcp::McpResponseTracker;
use crate::mcp::plugin_invoke::PluginInvokeRequest;
use crate::mcp::plugin_invoke::invoke_plugin_prompt_sender;
use smearor_mcp_server::CommandResponseWrapper;
use smearor_mcp_server::InvokePluginPromptParams;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use tokio::sync::mpsc::UnboundedSender;

/// Handle a plugin prompt invocation by sending the request via the broker
/// and awaiting the response with a 10-second timeout.
pub(crate) async fn handle_plugin_prompt(
    broker_sender: &UnboundedSender<FfiEnvelope>,
    response_tracker: &McpResponseTracker,
    command: CommandResponseWrapper<InvokePluginPromptParams>,
) {
    let receiver = response_tracker.register(command.params.correlation_id.clone());
    let send_result = invoke_plugin_prompt_sender(
        PluginInvokeRequest::builder()
            .broker_sender(broker_sender)
            .name(&command.params.name)
            .correlation_id(&command.params.correlation_id)
            .arguments(&command.params.arguments)
            .build(),
    );
    if send_result.is_ok() {
        let response_result = match tokio::time::timeout(tokio::time::Duration::from_secs(10), receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Plugin prompt invocation dropped".to_string()),
            Err(_) => Err("Plugin prompt invocation timed out".to_string()),
        };
        let _ = command.response.send(response_result);
    } else {
        let _ = command.response.send(send_result.map(|_| String::new()).map_err(|e| e.to_string()));
    }
}
