use crate::instance::LauncherInstance;
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
pub fn try_convert_string_to_typed_envelope(registry: &crate::json_converter::JsonConverterRegistry, envelope: &FfiEnvelope) -> Option<FfiEnvelope> {
    let string_type_id = smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String");
    if envelope.type_id != string_type_id || envelope.payload.is_null() {
        return None;
    }

    let payload_str = unsafe { (envelope.payload as *mut String).as_ref()? };
    let topic = envelope.topic.to_string();
    let sender_id = envelope.sender_id.to_string();
    registry.convert(&topic, &sender_id, payload_str)
}

impl LauncherInstance {
    pub fn handle_message(&self, envelope: FfiEnvelope) {
        let sender_id = envelope.sender_id.to_string();
        let topic = envelope.topic.to_string();

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

        if topic.starts_with("area.")
            && let Ok(area_manager) = self.area_manager.lock()
        {
            area_manager.route(&envelope);
        }

        // RequestClose — close only this instance's window
        if topic == "core.close" {
            if let Ok(window_guard) = self.window.lock() {
                if let Some(ref window) = *window_guard {
                    window.close();
                }
            }
            return;
        }

        if topic.starts_with("plugin.") {
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 {
                let target_plugin_id = parts[1];
                // Try raw ID first (backward compat / empty instance_id)
                let found = self.plugin_manager.plugins.get(target_plugin_id);
                // Then try namespaced ID
                let namespaced_id = format!("{}:{}", self.instance_id, target_plugin_id);
                let found = found.or_else(|| self.plugin_manager.plugins.get(&namespaced_id));
                if let Some(plugin) = found {
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

        // Broadcast status updates (e.g. audio.status, mpris.status) to all plugins
        if topic.ends_with(".status") {
            for r in self.plugin_manager.plugins.iter() {
                let plugin = r.value();
                unsafe {
                    plugin.on_message(envelope.clone());
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
