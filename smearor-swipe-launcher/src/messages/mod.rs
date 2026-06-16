use crate::application::LauncherApplication;
use gtk4::prelude::*;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageRouter;
use std::time::Duration;
use std::time::Instant;
use tracing::trace;
use tracing::warn;

impl LauncherApplication {
    pub fn handle_message(&self, envelope: FfiEnvelope) {
        let sender_id = envelope.sender_id.to_string();
        let topic = envelope.topic.to_string();
        let payload = envelope.payload.to_string();

        trace!("Event Broker: Received message from '{}' on topic '{}': {}", sender_id, topic, payload);

        // Rate-limit command topics to protect the broker from burst overload.
        if topic.ends_with(".command") || topic.ends_with(".status") {
            let now = Instant::now();
            let should_drop = {
                if let Ok(mut limiter) = self.topic_rate_limiter.lock() {
                    if let Some(last) = limiter.get(&topic) {
                        if now.duration_since(*last) < Duration::from_millis(30) {
                            true
                        } else {
                            limiter.insert(topic.clone(), now);
                            false
                        }
                    } else {
                        limiter.insert(topic.clone(), now);
                        false
                    }
                } else {
                    false
                }
            };
            if should_drop {
                warn!("Broker: Dropping burst command message on topic '{}'", topic);
                return;
            }
        }

        if topic.starts_with("area.")
            && let Ok(area_manager) = self.area_manager.lock()
        {
            area_manager.route(&envelope);
        }
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

        if topic.starts_with("plugin.") {
            let parts: Vec<&str> = topic.split('.').collect();
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

        if topic.starts_with("plugins.broadcast.") {
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
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 {
                let target_service_id = parts[1];
                trace!("target_service_id {target_service_id}");
                if let Some(service) = self.service_manager.services.get(target_service_id) {
                    trace!("Route message to service {target_service_id}");
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                }
            }
            if topic.ends_with(".status") {
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
        if topic.starts_with("services.broadcast.") {
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
