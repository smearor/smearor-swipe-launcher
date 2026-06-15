use crate::application::LauncherApplication;
use gtk4::prelude::*;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageRouter;
use tracing::debug;
use tracing::trace;

impl LauncherApplication {
    // pub fn handle_message(&self, envelope: FfiEnvelope, scrolled_window: &gtk4::ScrolledWindow, main_container: &gtk4::Box) {
    pub fn handle_message(&self, envelope: FfiEnvelope) {
        let sender_id = envelope.sender_id.to_string();
        let topic = envelope.topic.to_string();
        let payload = envelope.payload.to_string();

        debug!("Event Broker: Received message from '{}' on topic '{}': {}", sender_id, topic, payload);

        if topic.starts_with("area.")
            && let Ok(area_manager) = self.area_manager.lock()
        {
            area_manager.route(&envelope);
        }

        // // Area management messages
        // if topic == "area.open" {
        //     if let Ok(parsed) = serde_json::from_str::<Value>(&payload) {
        //         if let Some(area_id) = parsed.get("area_id").and_then(|v| v.as_str()) {
        //             info!("Opening area: {} from sender: {}", area_id, sender_id);
        //             if let Some(area_config) = self.config.get_area_config(area_id) {
        //                 if let Ok(mut area_manager) = self.area_manager.lock() {
        //                     if let Err(e) = area_manager.add_transient_area(area_id, area_config.clone(), self.main_container, Some(&sender_id)) {
        //                         error!("Failed to open area {}: {}", area_id, e);
        //                     } else {
        //                         info!("Successfully opened area {}", area_id);
        //                     }
        //                 }
        //             } else {
        //                 error!("Area config not found for: {}", area_id);
        //             }
        //         }
        //     }
        // }
        //
        // if topic == "area.close" {
        //     if let Ok(parsed) = serde_json::from_str::<Value>(&payload) {
        //         if let Some(area_id) = parsed.get("area_id").and_then(|v| v.as_str()) {
        //             info!("Closing area: {}", area_id);
        //             if let Ok(mut area_manager) = self.area_manager.lock() {
        //                 if let Err(e) = area_manager.remove_area(area_id, main_container) {
        //                     error!("Failed to close area {}: {}", area_id, e);
        //                 } else {
        //                     info!("Successfully closed area {}", area_id);
        //                 }
        //             }
        //         }
        //     }
        // }
        //
        // if topic == "area.add" {
        //     if let Ok(parsed) = serde_json::from_str::<Value>(&payload) {
        //         if let Some(area_id) = parsed.get("area_id").and_then(|v| v.as_str()) {
        //             info!("Adding area: {}", area_id);
        //             if let Some(area_config_value) = parsed.get("area_config") {
        //                 if let Ok(area_config) = serde_json::from_value::<AreaConfig>(area_config_value.clone()) {
        //                     if let Ok(mut area_manager) = self.area_manager.lock() {
        //                         if let Err(e) = area_manager.add_area_from_config(area_id, area_config, main_container) {
        //                             error!("Failed to add area {}: {}", area_id, e);
        //                         } else {
        //                             info!("Successfully added area {}", area_id);
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }
        //
        // if topic == "area.remove" {
        //     if let Ok(parsed) = serde_json::from_str::<Value>(&payload) {
        //         if let Some(area_id) = parsed.get("area_id").and_then(|v| v.as_str()) {
        //             info!("Removing area: {}", area_id);
        //             if let Ok(mut area_manager) = self.area_manager.lock() {
        //                 if let Err(e) = area_manager.remove_area(area_id, main_container) {
        //                     error!("Failed to remove area {}: {}", area_id, e);
        //                 } else {
        //                     info!("Successfully removed area {}", area_id);
        //                 }
        //             }
        //         }
        //     }
        // }

        // Example 1: RequestClose
        if topic == "core.close" {
            self.gtk_app.quit();
            return;
        }

        // // Example 2: ScrollToPosition / FocusWidget
        // if topic == "core.layout" {
        //     if let Ok(parsed) = serde_json::from_str::<Value>(&payload) {
        //         if parsed.get("action").and_then(|v| v.as_str()) == Some("FocusWidget") {
        //             if let Some(plugin_id) = parsed.get("plugin_id").and_then(|v| v.as_str()) {
        //                 info!("Plugin requested focus: {}", plugin_id);
        //                 if let Some(plugin_container) = scrolled_window.child().and_then(|c| c.downcast::<gtk4::Box>().ok()) {
        //                     // Find the widget corresponding to the plugin_id and scroll to it
        //                     // Here we can log and show how it performs.
        //                     debug!("Found plugin container: {:?}", plugin_container);
        //                 }
        //             }
        //         }
        //     }
        // }

        // Example 3: Routing to a specific plugin
        // Example 5: Plugin-to-Plugin direct signaling
        if topic.starts_with("plugin/") {
            let parts: Vec<&str> = topic.split('/').collect();
            if parts.len() >= 2 {
                let target_plugin_id = parts[1];
                if let Some(plugin) = self.plugin_manager.plugins.get(target_plugin_id) {
                    trace!("Routing message to plugin {target_plugin_id}");
                    unsafe {
                        plugin.on_message(envelope.clone());
                    }
                }
            }
        }

        // Example 4: Broadcasting to all plugins
        if topic.starts_with("plugins/broadcast/") {
            trace!("Broadcasting message to all loaded plugins");
            for r in self.plugin_manager.plugins.iter() {
                let plugin = r.value();
                unsafe {
                    plugin.on_message(envelope.clone());
                }
            }
        }

        // Routing to a specific service
        if topic.starts_with("service.") {
            let parts: Vec<&str> = topic.split('/').collect();
            if parts.len() >= 2 {
                let target_service_id = parts[1];
                if let Some(service) = self.service_manager.services.get(target_service_id) {
                    trace!("Route message to service {target_service_id}");
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                }
            }
            if topic.ends_with("/status") {
                trace!("Broadcasting service status update to all plugins");
                for r in self.plugin_manager.plugins.iter() {
                    let plugin = r.value();
                    unsafe {
                        plugin.on_message(envelope.clone());
                    }
                }
            }
        }

        // Broadcasting to all background services
        if topic.starts_with("services/broadcast/") {
            trace!("Broadcasting message to all background services");
            for r in self.service_manager.services.iter() {
                let service = r.value();
                unsafe {
                    service.on_message(envelope.clone());
                }
            }
        }
    }
}
