use crate::WidgetUpdateMessage;

smearor_swipe_launcher_plugin_api::impl_json_convertible!(WidgetUpdateMessageConverter, WidgetUpdateMessage, |json: serde_json::Value| {
    let plugin_id = json.get("plugin_id").and_then(|v| v.as_str()).unwrap_or("");
    let instance_id = json.get("instance_id").and_then(|v| v.as_str()).unwrap_or("");
    WidgetUpdateMessage::new(plugin_id, instance_id)
});

/// Register all JSON converter implementations for widget messages.
///
/// Call this once during startup.
pub fn register_json_converters(context: Option<smearor_swipe_launcher_plugin_api::FfiCoreContext>) {
    WidgetUpdateMessageConverter::register_in_host(context);
}
