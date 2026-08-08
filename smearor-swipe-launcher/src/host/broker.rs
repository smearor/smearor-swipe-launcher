use crate::context::GLOBAL_JSON_CONVERTER_REGISTRY;
use crate::messages::try_convert_string_to_typed_envelope;
use serde_json;
use smearor_model_compositor::TOPIC_CREATE_WORKSPACE;
use smearor_model_compositor::TOPIC_SWITCH_WORKSPACE;
use smearor_model_compositor::TOPIC_WORKSPACE_CHANGED;
use smearor_model_compositor::TOPIC_WORKSPACE_LIFECYCLE;
use smearor_model_compositor::TOPIC_WORKSPACE_SNAPSHOT;
use smearor_model_compositor::TOPIC_WORKSPACE_SNAPSHOT_REQUEST;
use smearor_model_instance_control::InstanceLoadMessage;
use smearor_model_instance_control::InstanceReloadMessage;
use smearor_model_instance_control::InstanceStartMessage;
use smearor_model_instance_control::InstanceStopMessage;
use smearor_model_instance_control::InstanceType as ModelInstanceType;
use smearor_model_instance_control::InstanceUnloadMessage;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_LOAD;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_RELOAD;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_START;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_STOP;
use smearor_model_instance_control::TOPIC_CORE_INSTANCE_UNLOAD;
use smearor_model_macropad::MacroPadConnectionStatus;
use smearor_model_macropad::MacroPadInputMessage;
use smearor_model_macropad::TOPIC_MACROPAD_COMMAND;
use smearor_model_macropad::TOPIC_MACROPAD_CONNECTION;
use smearor_model_macropad::TOPIC_MACROPAD_INPUT;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::InvokeToolResponse;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_widget::TOPIC_WIDGET_UPDATE;
use smearor_model_widget::WidgetUpdateMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use std::time::Duration;
use std::time::Instant;
use tracing::debug;
use tracing::error;
use tracing::trace;

use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;

use super::MACROPAD_COMPOUND_PRESS_WINDOW;
use super::MACROPAD_DOUBLE_PRESS_WINDOW;
use super::MACROPAD_LONGPRESS_THRESHOLD;
use super::TopicAction;

impl super::LauncherHost {
    pub(crate) fn route_message(&self, envelope: FfiEnvelope) {
        let mut target = envelope.target_instance_id.to_string();
        let topic = envelope.topic.to_string();
        trace!(
            "route_message: topic={} target={} ServiceManager ptr={:p} count={}",
            topic,
            target,
            self.service_manager.as_ref(),
            self.service_manager.services.len()
        );

        // Global MCP registration messages are routed to the shared registry
        // and forwarded only to the voice_assistant service, which builds and
        // maintains the tool catalog used for ReAct tool selection. Other
        // services do not handle RegisterToolMessage and would log noise.
        if topic == smearor_model_mcp::TOPIC_MCP_REGISTER_TOOL {
            MessageHandler::<FfiEnvelopePayload<RegisterToolMessage>>::handle_envelope_message(&self.mcp_registry, &envelope);
            trace!("Plugin registered a new MCP tool, list_changed notification deferred to SDK runtime");
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_REGISTER_RESOURCE {
            MessageHandler::<FfiEnvelopePayload<RegisterResourceMessage>>::handle_envelope_message(&self.mcp_registry, &envelope);
            trace!("Plugin registered a new MCP resource, list_changed notification deferred to SDK runtime");
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_REGISTER_PROMPT {
            MessageHandler::<FfiEnvelopePayload<RegisterPromptMessage>>::handle_envelope_message(&self.mcp_registry, &envelope);
            trace!("Plugin registered a new MCP prompt, list_changed notification deferred to SDK runtime");
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }

        // Global MCP invocation responses complete the pending response trackers
        // and are forwarded to the voice_assistant service which awaits
        // tool results via its own oneshot channels.
        if topic == smearor_model_mcp::TOPIC_MCP_TOOL_RESPONSE {
            trace!("Application: received mcp.tool.response, forwarding to tracker and voice_assistant");
            let response = unsafe { &*(envelope.payload as *const InvokeToolResponse) };
            let result = if response.error.is_empty() {
                Ok(response.result.to_string())
            } else {
                Err(response.error.to_string())
            };
            self.mcp_response_tracker.resolve(&response.correlation_id.to_string(), result);
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            } else {
                trace!("voice_assistant service not found, dropping tool response");
            }
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_RESOURCE_RESPONSE {
            let response = unsafe { &*(envelope.payload as *const InvokeResourceResponse) };
            let result = if response.error.is_empty() {
                Ok(response.contents.to_string())
            } else {
                Err(response.error.to_string())
            };
            self.mcp_response_tracker.resolve(&response.correlation_id.to_string(), result);
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }
        if topic == smearor_model_mcp::TOPIC_MCP_PROMPT_RESPONSE {
            let response = unsafe { &*(envelope.payload as *const InvokePromptResponse) };
            let messages_json = if response.error.is_empty() {
                let messages: Vec<serde_json::Value> = response
                    .messages
                    .iter()
                    .map(|m| serde_json::json!({"role": m.role.to_string(), "content": m.content.to_string()}))
                    .collect();
                serde_json::to_string(&serde_json::json!({
                    "description": "",
                    "messages": messages
                }))
                .unwrap_or_else(|_| "{}".to_string())
            } else {
                String::new()
            };
            let result = if response.error.is_empty() {
                Ok(messages_json)
            } else {
                Err(response.error.to_string())
            };
            self.mcp_response_tracker.resolve(&response.correlation_id.to_string(), result);
            if let Some(service) = self.service_manager.services.get("voice_assistant") {
                unsafe {
                    service.on_message(envelope);
                }
            }
            return;
        }

        // Try to convert a generic JSON-string payload into a typed message.
        // Services and instances share the same global registry.
        let mut envelope = envelope;
        if let Some(registry) = GLOBAL_JSON_CONVERTER_REGISTRY.get() {
            if let Some(converted) = try_convert_string_to_typed_envelope(registry, &envelope) {
                if !envelope.payload.is_null() {
                    if let Some(destroy) = envelope.destroy_payload {
                        unsafe {
                            (destroy)(envelope.payload);
                        }
                    }
                }
                envelope = converted;
            }
        }

        // Handle core.instance.* topics for dynamic instance management.
        if topic == TOPIC_CORE_INSTANCE_LOAD {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceLoadMessage) };
                let instance_id = msg.instance_id.to_string();
                let config_path = msg.config_path.to_string();
                let instance_type = match &msg.instance_type {
                    ModelInstanceType::Gtk => crate::instance::InstanceType::Gtk,
                    ModelInstanceType::Headless => crate::instance::InstanceType::Headless,
                    ModelInstanceType::Web => crate::instance::InstanceType::Web,
                };
                let persist = msg.persist;
                let response_topic = msg.response_topic.to_string();
                let result = self.load_instance(instance_id, &config_path, instance_type, persist, true);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }
        if topic == TOPIC_CORE_INSTANCE_START {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceStartMessage) };
                let instance_id = msg.instance_id.to_string();
                let response_topic = msg.response_topic.to_string();
                let result = self.start_instance(&instance_id);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }
        if topic == TOPIC_CORE_INSTANCE_STOP {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceStopMessage) };
                let instance_id = msg.instance_id.to_string();
                let response_topic = msg.response_topic.to_string();
                let result = self.stop_instance(&instance_id);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }
        if topic == TOPIC_CORE_INSTANCE_UNLOAD {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceUnloadMessage) };
                let instance_id = msg.instance_id.to_string();
                let response_topic = msg.response_topic.to_string();
                let result = self.unload_instance(&instance_id);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }
        if topic == TOPIC_CORE_INSTANCE_RELOAD {
            if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InstanceReloadMessage) };
                let instance_id = msg.instance_id.to_string();
                let config_path = msg.config_path.to_string();
                let response_topic = msg.response_topic.to_string();
                let result = self.reload_instance(&instance_id, &config_path);
                self.send_broker_response(&response_topic, &result);
            }
            return;
        }

        // Handle auto_start_topic / auto_stop_topic for event-driven lifecycle control.
        let topic_action = {
            if let Ok(registry) = self.topic_instance_registry.lock() {
                registry.get(&topic).map(|(id, action)| (id.clone(), *action))
            } else {
                None
            }
        };
        if let Some((instance_id, action)) = topic_action {
            match action {
                TopicAction::Start => {
                    debug!("auto_start_topic '{}' triggered start for instance '{}'", topic, instance_id);
                    let _ = self.start_instance(&instance_id);
                }
                TopicAction::Stop => {
                    debug!("auto_stop_topic '{}' triggered stop for instance '{}'", topic, instance_id);
                    let _ = self.stop_instance(&instance_id);
                }
            }
            return;
        }

        // Route MacroPad command messages to the matching service by target_instance_id.
        if topic == TOPIC_MACROPAD_COMMAND && !envelope.payload.is_null() {
            let target_service_id = envelope.target_instance_id.to_string();
            if let Some(service) = self.service_manager.services.get(&target_service_id) {
                trace!("Routing MacroPad command to service {}", target_service_id);
                unsafe {
                    service.on_message(envelope);
                }
            } else {
                debug!("No MacroPad service found for target_instance_id '{}'", target_service_id);
            }
            return;
        }

        // Handle MacroPad connection status: load/stop headless instances.
        if topic == TOPIC_MACROPAD_CONNECTION && !envelope.payload.is_null() {
            let msg = unsafe { &*(envelope.payload as *const MacroPadConnectionStatus) };
            let instance_id = msg.instance_id.to_string();
            let device_id = msg.device_id.to_string();
            let connected = msg.connected;
            let key_count = msg.key_count;
            let key_columns = msg.key_columns;
            let key_width = msg.key_width;
            let key_height = msg.key_height;
            let driver = msg.driver.to_string();
            debug!(
                "MacroPad connection status: instance={} device={} connected={} key_count={} key_columns={} key_width={} key_height={} driver={}",
                instance_id, device_id, connected, key_count, key_columns, key_width, key_height, driver
            );

            if connected {
                // Check if instance already exists.
                let already_exists = self.instances.lock().map(|instances| instances.contains_key(&instance_id)).unwrap_or(false);
                if !already_exists {
                    // Try to load a config file for this instance.
                    let config_path = format!("configs/launcher/{}.toml", instance_id);
                    if std::path::Path::new(&config_path).exists() {
                        match self.load_instance(instance_id.clone(), &config_path, crate::instance::InstanceType::Headless, false, true) {
                            Ok(_) => {
                                debug!("Loaded headless instance '{}' for MacroPad device '{}'", instance_id, device_id);
                                // Attach device metadata to the instance.
                                if let Ok(instances) = self.instances.lock() {
                                    if let Some(instance) = instances.get(&instance_id) {
                                        if let Ok(mut metadata) = instance.device_metadata.lock() {
                                            *metadata = Some(crate::instance::MacroPadDeviceMetadata {
                                                device_id: device_id.clone(),
                                                key_count,
                                                key_columns,
                                                key_width,
                                                key_height,
                                                driver: driver.clone(),
                                            });
                                        }
                                    }
                                }
                                // Render initial button images to the device.
                                self.render_buttons_to_device(&instance_id);
                            }
                            Err(e) => {
                                error!("Failed to load headless instance '{}': {}", instance_id, e);
                            }
                        }
                    } else {
                        trace!("No config file found for MacroPad instance '{}' at {}", instance_id, config_path);
                    }
                } else {
                    // Instance already exists (e.g. loaded from persistence at startup).
                    // Update device metadata and re-render buttons.
                    debug!("Instance '{}' already exists, updating device metadata and rendering buttons", instance_id);
                    if let Ok(instances) = self.instances.lock() {
                        if let Some(instance) = instances.get(&instance_id) {
                            if let Ok(mut metadata) = instance.device_metadata.lock() {
                                *metadata = Some(crate::instance::MacroPadDeviceMetadata {
                                    device_id: device_id.clone(),
                                    key_count,
                                    key_columns,
                                    key_width,
                                    key_height,
                                    driver: driver.clone(),
                                });
                            }
                        }
                    }
                    self.render_buttons_to_device(&instance_id);
                }
            } else {
                // Unload the instance on disconnect.
                let exists = self.instances.lock().map(|instances| instances.contains_key(&instance_id)).unwrap_or(false);
                if exists {
                    match self.unload_instance(&instance_id) {
                        Ok(_) => debug!("Unloaded headless instance '{}' for MacroPad device '{}'", instance_id, device_id),
                        Err(e) => error!("Failed to unload instance '{}': {}", instance_id, e),
                    }
                }
            }
            // Fall through to broadcast to all plugins (don't return).
        }

        // Handle MacroPad input: route to the matching instance by instance_id.
        if topic == TOPIC_MACROPAD_INPUT && !envelope.payload.is_null() {
            let msg = unsafe { &*(envelope.payload as *const MacroPadInputMessage) };
            let instance_id = msg.instance_id.to_string();
            let button_index = msg.button_index;
            let pressed = msg.pressed;
            debug!("MacroPad input: instance={} button={} pressed={}", instance_id, button_index, pressed);

            if pressed {
                // Record press start time for longpress/hold detection.
                if let Ok(mut press_times) = self.macropad_press_times.lock() {
                    press_times.insert((instance_id.clone(), button_index), Instant::now());
                }
                // If hold is configured for this button, dispatch hold_start immediately.
                if self.is_trigger_configured(&instance_id, button_index, "hold_topic") {
                    self.dispatch_macropad_action(&instance_id, button_index, "hold_start");
                }
                // Compound longpress: track span group presses.
                if let Some((span_group, group_buttons)) = self.get_span_group_for_button(&instance_id, button_index) {
                    let now = Instant::now();
                    if let Ok(mut compound) = self.macropad_compound_presses.lock() {
                        let entry = compound.entry((instance_id.clone(), span_group.clone())).or_default();
                        // Only add if not already pressed.
                        if !entry.iter().any(|(b, _)| *b == button_index) {
                            entry.push((button_index, now));
                        }
                        // Check if 2+ buttons are pressed within the compound window.
                        let recent_count = entry.iter().filter(|(_, t)| now.duration_since(*t) <= MACROPAD_COMPOUND_PRESS_WINDOW).count();
                        if recent_count >= 2 {
                            // Schedule compound longpress check after threshold.
                            let self_clone = self.clone();
                            let key = (instance_id.clone(), span_group.clone());
                            gtk4::glib::timeout_add_local_once(MACROPAD_LONGPRESS_THRESHOLD, move || {
                                if let Ok(compound) = self_clone.macropad_compound_presses.lock() {
                                    if let Some(presses) = compound.get(&key) {
                                        // Buttons are removed from tracking on release.
                                        // If 2+ are still tracked after 500ms, they're all held.
                                        if presses.len() >= 2 {
                                            for &btn in group_buttons.iter() {
                                                self_clone.dispatch_macropad_action(&key.0, btn, "compound_longpress");
                                            }
                                            if let Ok(mut dispatched) = self_clone.macropad_compound_dispatched.lock() {
                                                dispatched.insert(key.clone(), ());
                                            }
                                            debug!("Compound longpress triggered for span group '{}' in instance '{}'", key.1, key.0);
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            } else {
                // On release: determine action based on hold duration and configured triggers.
                let press_start = if let Ok(mut press_times) = self.macropad_press_times.lock() {
                    press_times.remove(&(instance_id.clone(), button_index))
                } else {
                    None
                };

                let duration = press_start.map(|start| start.elapsed()).unwrap_or(Duration::ZERO);

                // Check if this button is part of a span group and if a compound
                // longpress was triggered (all buttons held >= 500ms).
                let compound_triggered = if let Some((span_group, _)) = self.get_span_group_for_button(&instance_id, button_index) {
                    // Remove this button from compound tracking regardless.
                    if let Ok(mut compound) = self.macropad_compound_presses.lock() {
                        if let Some(presses) = compound.get_mut(&(instance_id.clone(), span_group.clone())) {
                            presses.retain(|(b, _)| *b != button_index);
                        }
                        if compound.get(&(instance_id.clone(), span_group.clone())).map_or(false, |p| p.is_empty()) {
                            compound.remove(&(instance_id.clone(), span_group.clone()));
                        }
                    }
                    // Check if compound longpress was actually dispatched for this span group.
                    let dispatched = if let Ok(mut dispatched) = self.macropad_compound_dispatched.lock() {
                        dispatched.remove(&(instance_id.clone(), span_group)).is_some()
                    } else {
                        false
                    };
                    if dispatched && duration >= MACROPAD_LONGPRESS_THRESHOLD {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if compound_triggered {
                    debug!(
                        "MacroPad: compound longpress suppressed individual action for instance '{}' button {} ({}ms)",
                        instance_id,
                        button_index,
                        duration.as_millis()
                    );
                    // Still dispatch hold_stop if hold was configured.
                    if self.is_trigger_configured(&instance_id, button_index, "hold_topic") {
                        self.dispatch_macropad_action(&instance_id, button_index, "hold_stop");
                    }
                } else {
                    let hold_configured = self.is_trigger_configured(&instance_id, button_index, "hold_topic");

                    // If hold is configured, dispatch hold_stop on release.
                    if hold_configured {
                        self.dispatch_macropad_action(&instance_id, button_index, "hold_stop");
                        // If hold is configured and press was short, suppress click
                        // (the hold_start + hold_stop pair already fired).
                        if duration < MACROPAD_LONGPRESS_THRESHOLD {
                            debug!(
                                "MacroPad: hold suppressed click for instance '{}' button {} ({}ms)",
                                instance_id,
                                button_index,
                                duration.as_millis()
                            );
                        } else {
                            // Hold was active but press was >= 500ms: dispatch longpress.
                            self.dispatch_macropad_action(&instance_id, button_index, "longpress");
                        }
                    } else if duration >= MACROPAD_LONGPRESS_THRESHOLD {
                        // No hold configured, long press.
                        self.dispatch_macropad_action(&instance_id, button_index, "longpress");
                    } else {
                        // Short press without hold: check for double press.
                        let double_press_configured = self.is_trigger_configured(&instance_id, button_index, "double_press_topic");

                        if double_press_configured && duration < MACROPAD_DOUBLE_PRESS_WINDOW {
                            // Check for pending click (double press detection).
                            let has_pending = if let Ok(mut pending) = self.macropad_pending_clicks.lock() {
                                pending.remove(&(instance_id.clone(), button_index)).is_some()
                            } else {
                                false
                            };

                            if has_pending {
                                // Second click within window: dispatch double_press.
                                self.dispatch_macropad_action(&instance_id, button_index, "double_press");
                            } else {
                                // First click: record as pending and schedule delayed click dispatch.
                                if let Ok(mut pending) = self.macropad_pending_clicks.lock() {
                                    pending.insert((instance_id.clone(), button_index), Instant::now());
                                }
                                let self_clone = self.clone();
                                let key = (instance_id.clone(), button_index);
                                gtk4::glib::timeout_add_local_once(MACROPAD_DOUBLE_PRESS_WINDOW, move || {
                                    // If the pending click is still there (no second press came),
                                    // dispatch it as a normal click.
                                    let still_pending = if let Ok(mut pending) = self_clone.macropad_pending_clicks.lock() {
                                        pending.remove(&key).is_some()
                                    } else {
                                        false
                                    };
                                    if still_pending {
                                        self_clone.dispatch_macropad_action(&key.0, key.1, "click");
                                    }
                                });
                            }
                        } else {
                            // Normal click (duration 300-500ms, or no double_press configured).
                            self.dispatch_macropad_action(&instance_id, button_index, "click");
                        }
                    }
                }
            }
            // Fall through to broadcast to all plugins (don't return).
        }

        // Route service.* topics to the shared ServiceManager, except for known
        // outbound topics that services broadcast to widgets.
        if topic.starts_with("service.")
            && !topic.ends_with(".status")
            && !topic.ends_with(".scan_results")
            && !topic.ends_with(".vpn_profiles")
            && !topic.contains(".response.")
            && topic != TOPIC_MACROPAD_INPUT
            && topic != TOPIC_MACROPAD_CONNECTION
        {
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 {
                let target_service_id = parts[1];
                if let Some(service) = self.service_manager.services.get(target_service_id) {
                    trace!("Routing message to service {}", target_service_id);
                    unsafe {
                        service.on_message(envelope);
                    }
                }
            }
            return;
        }

        // MCP invocation requests are routed exclusively to the owning service
        // or plugin, as determined by the McpRegistry. This prevents unrelated
        // services from responding with errors for URIs/tools they don't own.
        if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE
            || topic == smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL
            || topic == smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT
        {
            let plugin_id = if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE {
                if !envelope.payload.is_null() {
                    let msg = unsafe { &*(envelope.payload as *const InvokeResourceMessage) };
                    let uri = msg.uri.to_string();
                    self.mcp_registry.list_resources().iter().find(|r| r.uri == uri).map(|r| r.plugin_id.clone())
                } else {
                    None
                }
            } else if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_PROMPT {
                if !envelope.payload.is_null() {
                    let msg = unsafe { &*(envelope.payload as *const InvokePromptMessage) };
                    let name = msg.name.to_string();
                    self.mcp_registry.list_prompts().iter().find(|p| p.name == name).map(|p| p.plugin_id.clone())
                } else {
                    None
                }
            } else if !envelope.payload.is_null() {
                let msg = unsafe { &*(envelope.payload as *const InvokeToolMessage) };
                let name = msg.name.to_string();
                self.mcp_registry.list_tools().iter().find(|t| t.name == name).map(|t| t.plugin_id.clone())
            } else {
                None
            };

            if let Some(plugin_id) = plugin_id {
                debug!("routing {} to plugin_id={}", topic, plugin_id);
                // Try service first
                if let Some(service) = self.service_manager.services.get(&plugin_id) {
                    debug!("sending {} to service {}", topic, plugin_id);
                    unsafe {
                        service.on_message(envelope);
                    }
                    return;
                }
                // Route to specific plugin in instances via targeted target_instance_id
                let mut targeted_envelope = envelope;
                targeted_envelope.target_instance_id = stabby::string::String::from(plugin_id);
                if let Ok(instances) = self.instances.lock() {
                    for instance in instances.values() {
                        instance.handle_message(targeted_envelope.clone());
                    }
                }
                return;
            }

            // Fallback: no plugin owns this invocation. Try the launcher core's
            // built-in MCP capabilities (area tools, resources, etc.) before giving up.
            debug!("no plugin owner for {}, trying core MCP capabilities", topic);
            if let Ok(sender_guard) = self.mcp_command_sender.lock() {
                if let Some(sender) = sender_guard.as_ref().cloned() {
                    let broker_sender = self.broker_sender.clone();
                    if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL && !envelope.payload.is_null() {
                        let msg = unsafe { &*(envelope.payload as *const InvokeToolMessage) };
                        let name = msg.name.to_string();
                        let correlation_id = msg.correlation_id.to_string();
                        let arguments = serde_json::from_str(&msg.arguments.to_string()).unwrap_or(serde_json::Value::Null);
                        tokio::spawn(async move {
                            let tools = smearor_mcp_server::tools::core_tools();
                            let result = smearor_mcp_server::tools::invoke_tool_sdk(&tools, sender, &name, Some(&arguments)).await;
                            let response = match result {
                                Ok(text) => InvokeToolResponse::success(&correlation_id, &text),
                                Err(error) => InvokeToolResponse::error(&correlation_id, &error),
                            };
                            let payload_ptr = box_payload(response);
                            let _ = broker_sender.send(
                                FfiEnvelope::builder()
                                    .sender_id("launcher-core")
                                    .target_instance_id("*")
                                    .topic(InvokeToolResponse::topic())
                                    .type_id(FfiEnvelopePayload::<InvokeToolResponse>::TYPE_ID)
                                    .payload(payload_ptr)
                                    .destroy_payload(Some(default_destroy_payload))
                                    .clone_payload(Some(default_clone_payload::<InvokeToolResponse>))
                                    .build(),
                            );
                        });
                        return;
                    }
                    if topic == smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE && !envelope.payload.is_null() {
                        let msg = unsafe { &*(envelope.payload as *const InvokeResourceMessage) };
                        let uri = msg.uri.to_string();
                        let correlation_id = msg.correlation_id.to_string();
                        tokio::spawn(async move {
                            let resources = smearor_mcp_server::resources::core_resources();
                            let result = smearor_mcp_server::resources::read_resource_sdk(&resources, sender, &uri).await;
                            let response = match result {
                                Ok((contents, _mime_type)) => InvokeResourceResponse::success(&correlation_id, &contents),
                                Err(error) => InvokeResourceResponse::error(&correlation_id, &error),
                            };
                            let payload_ptr = box_payload(response);
                            let _ = broker_sender.send(
                                FfiEnvelope::builder()
                                    .sender_id("launcher-core")
                                    .target_instance_id("*")
                                    .topic(InvokeResourceResponse::topic())
                                    .type_id(FfiEnvelopePayload::<InvokeResourceResponse>::TYPE_ID)
                                    .payload(payload_ptr)
                                    .destroy_payload(Some(default_destroy_payload))
                                    .clone_payload(Some(default_clone_payload::<InvokeResourceResponse>))
                                    .build(),
                            );
                        });
                        return;
                    }
                }
            }

            // Last resort: broadcast to all services so a handler can respond.
            debug!("no handler for {}, broadcasting to all services", topic);
            let service_ids: Vec<String> = self.service_manager.services.iter().map(|s| s.key().to_string()).collect();
            for service_id in service_ids {
                if let Some(service) = self.service_manager.services.get(&service_id) {
                    debug!("sending {} to service {} (broadcast)", topic, service_id);
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                }
            }
            // Fall through to broadcast to instances below
        }

        // Route compositor command topics (Widget -> Service) to all services.
        // Services that implement the relevant MessageHandler will process them;
        // others will ignore them via on_message dispatch.
        if topic == TOPIC_SWITCH_WORKSPACE || topic == TOPIC_CREATE_WORKSPACE || topic == TOPIC_WORKSPACE_SNAPSHOT_REQUEST {
            let service_ids: Vec<String> = self.service_manager.services.iter().map(|s| s.key().to_string()).collect();
            for service_id in service_ids {
                if let Some(service) = self.service_manager.services.get(&service_id) {
                    trace!("Routing compositor command {} to service {}", topic, service_id);
                    unsafe {
                        service.on_message(envelope.clone());
                    }
                }
            }
            return;
        }

        // Broadcast to all instances and services (used by shared services for status updates).
        // Services also need status messages for cross-service features like DoA VAD triggering
        // and audio ducking. Services without a matching MessageHandler ignore the message.
        if target == "*" || (target.is_empty() && topic.ends_with(".status")) {
            if topic.starts_with("service.") {
                let service_ids: Vec<String> = self.service_manager.services.iter().map(|s| s.key().to_string()).collect();
                for service_id in service_ids {
                    if let Some(service) = self.service_manager.services.get(&service_id) {
                        trace!("Forwarding status {} to service {}", topic, service_id);
                        unsafe {
                            service.on_message(envelope.clone());
                        }
                    }
                }
            }
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
                    self.forward_to_websockets(&instance.instance_id, &envelope);
                }
            }
            return;
        }

        // Broadcast service response topics (e.g. service.http.response.*)
        // to all instances so widgets can react to HTTP responses.
        if target.is_empty() && topic.starts_with("service.") && topic.contains(".response.") {
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
                    self.forward_to_websockets(&instance.instance_id, &envelope);
                }
            }
            return;
        }

        // Broadcast workspace events from compositor services to all instances.
        if target.is_empty() && topic == TOPIC_WORKSPACE_CHANGED {
            let instance_ids: Vec<String> = if let Ok(instances) = self.instances.lock() {
                instances.values().map(|i| i.instance_id.clone()).collect()
            } else {
                Vec::new()
            };
            for id in &instance_ids {
                if let Ok(instances) = self.instances.lock() {
                    if let Some(instance) = instances.get(id) {
                        instance.handle_message(envelope.clone());
                        self.forward_to_websockets(&instance.instance_id, &envelope);
                    }
                }
                // After a workspace change, re-render buttons for headless MacroPad instances
                // since on_workspace_changed may have rebuilt areas with different plugins.
                self.render_buttons_to_device(id);
            }
            return;
        }

        // Broadcast workspace snapshot and lifecycle events to all instances.
        if target.is_empty() && (topic == TOPIC_WORKSPACE_SNAPSHOT || topic == TOPIC_WORKSPACE_LIFECYCLE) {
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
                    self.forward_to_websockets(&instance.instance_id, &envelope);
                }
            }
            return;
        }

        // Detect implicit cross-instance addressing when a plugin sends an area_id
        // containing a colon (e.g. "side2:submenu") without setting target_instance_id.
        if target.is_empty() && topic.starts_with("area.") {
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 && parts[1].contains(':') {
                let (instance, area) = parts[1].split_once(':').unwrap_or(("", ""));
                if !instance.is_empty() {
                    target = instance.to_string();
                    // Reconstruct topic with local area_id for the target instance
                    let new_topic = format!(
                        "area.{}{}",
                        area,
                        if parts.len() > 2 {
                            format!(".{}", &parts[2..].join("."))
                        } else {
                            String::new()
                        }
                    );
                    let mut envelope = envelope;
                    envelope.topic = stabby::string::String::from(new_topic);
                    if let Ok(instances) = self.instances.lock() {
                        if let Some(target_instance) = instances.get(&target) {
                            target_instance.handle_message(envelope);
                        } else {
                            debug!("Unknown target instance '{}' for area message, dropping", target);
                        }
                    }
                    // Re-render buttons for headless MacroPad instances after area switch.
                    self.render_buttons_to_device(&target);

                    // For web instances, push updated widgets via WebSocket.
                    if let Ok(instances) = self.instances.lock() {
                        if let Some(instance) = instances.get(&target) {
                            if instance.instance_type == crate::instance::InstanceType::Web {
                                let widgets_html = crate::web::routes::render_all_widgets_html(instance);
                                self.send_web_widgets_update(&target, &widgets_html);
                            }
                        }
                    }
                    return;
                }
            }
        }

        // Route to a specific instance
        let target_instance = if target.is_empty() {
            // Extract instance from sender_id (format: "instance_id:plugin_id")
            envelope.sender_id.to_string().split(':').next().unwrap_or("").to_string()
        } else {
            target
        };

        // Extract widget.update payload before envelope is moved.
        let widget_update_info: Option<(String, String)> = if topic == TOPIC_WIDGET_UPDATE && !envelope.payload.is_null() {
            let msg = unsafe { &*(envelope.payload as *const WidgetUpdateMessage) };
            Some((msg.instance_id.to_string(), msg.plugin_id.to_string()))
        } else {
            None
        };

        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(&target_instance) {
                self.forward_to_websockets(&target_instance, &envelope);
                instance.handle_message(envelope);
            } else {
                debug!("Unknown target instance '{}', dropping message", target_instance);
            }
        }

        // After area open/close on a headless MacroPad instance, re-render buttons.
        if topic == "area.open" || topic == "area.close" {
            self.render_buttons_to_device(&target_instance);

            // For web instances, render the new visible area's widgets and
            // push them via WebSocket so the frontend can re-render.
            if let Ok(instances) = self.instances.lock() {
                if let Some(instance) = instances.get(&target_instance) {
                    if instance.instance_type == crate::instance::InstanceType::Web {
                        let widgets_html = crate::web::routes::render_all_widgets_html(instance);
                        self.send_web_widgets_update(&target_instance, &widgets_html);
                    }
                }
            }
        }

        // Handle widget.update: a widget's visual state changed, re-render it.
        // Prefer the instance_id from the message payload, but fall back to
        // target_instance (derived from sender_id) when the payload's
        // instance_id is empty — widgets can't know their own instance_id
        // because meta.id doesn't include the instance prefix.
        if let Some((msg_instance_id, plugin_id)) = widget_update_info {
            let render_instance = if msg_instance_id.is_empty() { &target_instance } else { &msg_instance_id };
            trace!("Widget update: instance='{}' plugin='{}'", render_instance, plugin_id);
            self.render_single_button_to_device(render_instance, &plugin_id);

            // For web instances, re-render the single widget and push via WebSocket.
            if let Ok(instances) = self.instances.lock() {
                if let Some(instance) = instances.get(render_instance) {
                    if instance.instance_type == crate::instance::InstanceType::Web {
                        let namespaced_id = format!("{}:{}", render_instance, plugin_id);
                        let html = crate::web::routes::render_single_widget_html(instance, &namespaced_id);
                        self.send_widget_update(render_instance, &namespaced_id, &html);
                    }
                }
            }
        }
    }

    /// Forward a broker message to WebSocket clients if the target instance
    /// is a Web instance with an active WebSocket channel.
    fn forward_to_websockets(&self, instance_id: &str, envelope: &FfiEnvelope) {
        if instance_id.is_empty() {
            return;
        }

        let Ok(web_server_guard) = self.web_server.lock() else {
            return;
        };
        let Some(ref web_server) = *web_server_guard else {
            return;
        };

        let ws_manager = web_server.ws_manager();
        if ws_manager.get_sender(instance_id).is_none() {
            return;
        }

        let payload = crate::web::routes::extract_payload_as_json(envelope);
        let update = crate::web::web_update::WebUpdate {
            instance_id: instance_id.to_string(),
            topic: envelope.topic.to_string(),
            sender_id: envelope.sender_id.to_string(),
            payload,
        };
        ws_manager.broadcast(&update);
    }

    /// Push rendered widgets HTML to WebSocket clients of a web instance.
    fn send_web_widgets_update(&self, instance_id: &str, widgets_html: &str) {
        let Ok(web_server_guard) = self.web_server.lock() else {
            return;
        };
        let Some(ref web_server) = *web_server_guard else {
            return;
        };

        let ws_manager = web_server.ws_manager();
        if ws_manager.get_sender(instance_id).is_none() {
            return;
        }

        let update = crate::web::web_update::WebUpdate {
            instance_id: instance_id.to_string(),
            topic: "area.changed".to_string(),
            sender_id: "system".to_string(),
            payload: serde_json::json!({ "widgets_html": widgets_html }).to_string(),
        };
        ws_manager.broadcast(&update);
    }

    /// Push a single widget's updated HTML to WebSocket clients.
    fn send_widget_update(&self, instance_id: &str, plugin_id: &str, html: &str) {
        let Ok(web_server_guard) = self.web_server.lock() else {
            return;
        };
        let Some(ref web_server) = *web_server_guard else {
            return;
        };

        let ws_manager = web_server.ws_manager();
        if ws_manager.get_sender(instance_id).is_none() {
            return;
        }

        let update = crate::web::web_update::WebUpdate {
            instance_id: instance_id.to_string(),
            topic: "widget.update".to_string(),
            sender_id: "system".to_string(),
            payload: serde_json::json!({ "plugin_id": plugin_id, "html": html }).to_string(),
        };
        ws_manager.broadcast(&update);
    }
}
