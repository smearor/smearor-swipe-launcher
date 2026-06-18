use crate::application::LauncherApplication;
use gtk4::prelude::*;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageRouter;
use std::time::Duration;
use std::time::Instant;
use tracing::trace;
use tracing::warn;

/// Attempts to convert a String payload to a typed message envelope using
/// the JSON converter registry.
///
/// Generic widgets (e.g. button) send plain JSON string payloads. The Host
/// uses the registry (populated via `register_json_converter!`) to convert
/// those strings into typed messages based on the message topic.
fn try_convert_string_to_typed_envelope(registry: &crate::json_converter::JsonConverterRegistry, envelope: &FfiEnvelope) -> Option<FfiEnvelope> {
    let string_type_id = smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String");
    if envelope.type_id != string_type_id || envelope.payload.is_null() {
        return None;
    }

    let payload_str = unsafe { (envelope.payload as *mut String).as_ref()? };
    let topic = envelope.topic.to_string();
    let sender_id = envelope.sender_id.to_string();
    registry.convert(&topic, &sender_id, payload_str)
}

impl LauncherApplication {
    pub fn handle_message(&self, envelope: FfiEnvelope) {
        let sender_id = envelope.sender_id.to_string();
        let topic = envelope.topic.to_string();

        println!("HOST handle_message: sender={} topic={} type_id={}", sender_id, topic, envelope.type_id);
        trace!("Event Broker: Received message from '{}' on topic '{}' (type_id={})", sender_id, topic, envelope.type_id);

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

        // Try to convert a generic JSON-string payload into a typed message
        // for *any* topic. Plugins register their converters at load time via
        // the FFI callback, so the Host can remain fully generic.
        let mut envelope = envelope;
        if let Some(converted) = try_convert_string_to_typed_envelope(&self.json_converter_registry, &envelope) {
            // Destroy the original String payload before replacing the envelope
            if !envelope.payload.is_null() {
                if let Some(destroy) = envelope.destroy_payload {
                    unsafe {
                        (destroy)(envelope.payload);
                    }
                }
            }
            envelope = converted;
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
            println!("HOST broadcasting to all plugins");
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
            println!("HOST routing to service: parts={:?}", parts);
            if parts.len() >= 2 {
                let target_service_id = parts[1];
                println!("HOST target_service_id={}", target_service_id);
                if let Some(service) = self.service_manager.services.get(target_service_id) {
                    println!("HOST calling service.on_message for {}", target_service_id);
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                } else {
                    println!("HOST service {} NOT FOUND", target_service_id);
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

        // Destroy the payload after all handlers have processed the message
        if !envelope.payload.is_null() {
            if let Some(destroy) = envelope.destroy_payload {
                unsafe {
                    (destroy)(envelope.payload);
                }
            }
        }
    }
}
