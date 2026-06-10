use crate::application::LauncherApplication;
use gtk4::prelude::*;
use smearor_plugin_api::FfiEnvelope;

impl LauncherApplication {
    pub fn handle_message(&self, envelope: FfiEnvelope, scrolled_window: &gtk4::ScrolledWindow) {
        let sender_id = envelope.sender_id.to_string();
        let topic = envelope.topic.to_string();
        let payload = envelope.payload.to_string();

        tracing::debug!("Event Broker: Received message from '{}' on topic '{}': {}", sender_id, topic, payload);

        // Example 1: RequestClose
        if topic == "core/control" {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&payload) {
                if parsed.get("action").and_then(|v| v.as_str()) == Some("RequestClose") {
                    tracing::info!("Plugin requested close");
                    self.gtk_app.quit();
                    return;
                }
            }
        }

        // Example 2: ScrollToPosition / FocusWidget
        if topic == "core/layout" {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&payload) {
                if parsed.get("action").and_then(|v| v.as_str()) == Some("FocusWidget") {
                    if let Some(plugin_id) = parsed.get("plugin_id").and_then(|v| v.as_str()) {
                        tracing::info!("Plugin requested focus: {}", plugin_id);
                        if let Some(plugin_container) = scrolled_window.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
                            // Find the widget corresponding to the plugin_id and scroll to it
                            // Here we can log and show how it performs.
                            tracing::debug!("Found plugin container: {:?}", plugin_container);
                        }
                    }
                }
            }
        }

        // Example 3: Routing to a specific plugin
        // Example 5: Plugin-to-Plugin direct signaling
        if topic.starts_with("plugin/") {
            let parts: Vec<&str> = topic.split('/').collect();
            if parts.len() >= 2 {
                let target_plugin_id = parts[1];
                if let Some(plugin) = self.plugin_manager.plugins.get(target_plugin_id) {
                    tracing::debug!("Event Broker: Routing message to specific plugin: {}", target_plugin_id);
                    unsafe {
                        plugin.on_message(envelope.clone());
                    }
                }
            }
        }

        // Example 4: Broadcasting to all plugins
        if topic.starts_with("plugins/broadcast/") {
            tracing::debug!("Event Broker: Broadcasting message to all loaded plugins");
            for r in self.plugin_manager.plugins.iter() {
                let plugin = r.value();
                unsafe {
                    plugin.on_message(envelope.clone());
                }
            }
        }

        // Routing to a specific background service
        if topic.starts_with("service/") {
            let parts: Vec<&str> = topic.split('/').collect();
            if parts.len() >= 2 {
                let target_service_id = parts[1];
                if let Some(service) = self.service_manager.services.get(target_service_id) {
                    tracing::debug!("Event Broker: Routing message to specific service: {}", target_service_id);
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                }
            }
        }

        // Broadcasting to all background services
        if topic.starts_with("services/broadcast/") {
            tracing::debug!("Event Broker: Broadcasting message to all background services");
            for r in self.service_manager.services.iter() {
                let service = r.value();
                unsafe {
                    service.on_message(envelope.clone());
                }
            }
        }
    }
}
