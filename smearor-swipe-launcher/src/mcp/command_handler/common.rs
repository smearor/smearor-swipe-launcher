use crate::area::instance_area_manager::InstanceAreaManager;
use crate::host::LauncherHost;

/// Helper to access the first available area manager.
pub(crate) fn with_first_area_manager<F, T>(host: &LauncherHost, callback: F) -> Result<T, String>
where
    F: FnOnce(&InstanceAreaManager) -> Result<T, String>,
{
    let instances = host.instances.lock().map_err(|_| "Failed to lock instances")?;
    let first_instance = instances.values().next().ok_or("No launcher instance available")?;
    let area_manager = first_instance.area_manager.lock().map_err(|_| "Failed to lock area manager")?;
    callback(&area_manager)
}

/// Send a single message to the broker on behalf of the MCP server.
pub(crate) fn send_mcp_message(host: &LauncherHost, topic: String, payload: serde_json::Value, target_instance_id: Option<String>) -> Result<String, String> {
    let payload_json = payload.to_string();
    let payload_ptr = smearor_swipe_launcher_plugin_api::box_payload(payload_json);
    let envelope = smearor_swipe_launcher_plugin_api::FfiEnvelope::builder()
        .sender_id("mcp-server")
        .target_instance_id(target_instance_id.unwrap_or_default())
        .topic(topic)
        .type_id(smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String"))
        .payload(payload_ptr)
        .destroy_payload(Some(smearor_swipe_launcher_plugin_api::default_destroy_payload))
        .clone_payload(Some(smearor_swipe_launcher_plugin_api::default_clone_payload::<String>))
        .build();
    host.broker_sender.send(envelope).map_err(|e| format!("Failed to send message: {}", e))?;
    Ok("Message sent".to_string())
}
