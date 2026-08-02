use crate::InstanceLifecycleEvent;
use crate::InstanceLoadMessage;
use crate::InstanceReloadMessage;
use crate::InstanceStatusMessage;
use crate::InstanceStopMessage;
use crate::InstanceType;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(InstanceLoadMessageConverter, InstanceLoadMessage, |json: serde_json::Value| {
    let instance_id = json.get("instance_id").and_then(|v| v.as_str()).unwrap_or("");
    let config_path = json.get("config_path").and_then(|v| v.as_str()).unwrap_or("");
    let instance_type = InstanceType::from_str(json.get("instance_type").and_then(|v| v.as_str()).unwrap_or("gtk")).unwrap_or(InstanceType::Gtk);
    let response_topic = json.get("response_topic").and_then(|v| v.as_str()).unwrap_or("");
    InstanceLoadMessage::new(instance_id, config_path, instance_type, response_topic)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(InstanceStopMessageConverter, InstanceStopMessage, |json: serde_json::Value| {
    let instance_id = json.get("instance_id").and_then(|v| v.as_str()).unwrap_or("");
    let response_topic = json.get("response_topic").and_then(|v| v.as_str()).unwrap_or("");
    InstanceStopMessage::new(instance_id, response_topic)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(InstanceReloadMessageConverter, InstanceReloadMessage, |json: serde_json::Value| {
    let instance_id = json.get("instance_id").and_then(|v| v.as_str()).unwrap_or("");
    let config_path = json.get("config_path").and_then(|v| v.as_str()).unwrap_or("");
    let response_topic = json.get("response_topic").and_then(|v| v.as_str()).unwrap_or("");
    InstanceReloadMessage::new(instance_id, config_path, response_topic)
});

smearor_swipe_launcher_plugin_api::impl_json_convertible!(InstanceStatusMessageConverter, InstanceStatusMessage, |json: serde_json::Value| {
    let instance_id = json.get("instance_id").and_then(|v| v.as_str()).unwrap_or("");
    let event = InstanceLifecycleEvent::from_str(json.get("event").and_then(|v| v.as_str()).unwrap_or("Loaded")).unwrap_or(InstanceLifecycleEvent::Loaded);
    InstanceStatusMessage::new(instance_id, event)
});

/// Register all JSON converter implementations for instance-control messages.
///
/// Call this once during startup.
pub fn register_json_converters(context: Option<smearor_swipe_launcher_plugin_api::FfiCoreContext>) {
    InstanceLoadMessageConverter::register_in_host(context);
    InstanceStopMessageConverter::register_in_host(context);
    InstanceReloadMessageConverter::register_in_host(context);
    InstanceStatusMessageConverter::register_in_host(context);
}
