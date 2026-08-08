use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use tokio::sync::mpsc::UnboundedSender;

use super::error::PluginInvokeError;
use super::request::PluginInvokeRequest;

fn invoke_plugin<T>(broker_sender: &UnboundedSender<FfiEnvelope>, message: T, correlation_id: &str, label: &'static str) -> Result<(), PluginInvokeError>
where
    T: TypedMessage + MessageTopic + Clone,
{
    let payload_ptr = box_payload(message);
    let envelope = FfiEnvelope::builder()
        .sender_id("mcp-server")
        .target_instance_id("*")
        .topic(T::topic())
        .type_id(T::TYPE_ID)
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<T>))
        .build();
    broker_sender.send(envelope).map_err(|e| PluginInvokeError::SendFailed { label, source: e })?;
    Ok(())
}

pub fn invoke_plugin_tool_sender(request: PluginInvokeRequest<'_>) -> Result<(), PluginInvokeError> {
    invoke_plugin(
        request.broker_sender,
        InvokeToolMessage::new(request.name, request.correlation_id, &request.arguments.to_string()),
        request.correlation_id,
        "tool",
    )
}

pub fn invoke_plugin_resource_sender(broker_sender: &UnboundedSender<FfiEnvelope>, uri: &str, correlation_id: &str) -> Result<(), PluginInvokeError> {
    invoke_plugin(broker_sender, InvokeResourceMessage::new(uri, correlation_id), correlation_id, "resource")
}

pub fn invoke_plugin_prompt_sender(request: PluginInvokeRequest<'_>) -> Result<(), PluginInvokeError> {
    invoke_plugin(
        request.broker_sender,
        InvokePromptMessage::new(request.name, request.correlation_id, &request.arguments.to_string()),
        request.correlation_id,
        "prompt",
    )
}
