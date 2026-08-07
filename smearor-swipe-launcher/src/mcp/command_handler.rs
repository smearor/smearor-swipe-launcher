use crate::application::LauncherHost;
use crate::area::instance_area_manager::InstanceAreaManager;
use crate::mcp::plugin_invoker::invoke_plugin_prompt_sender;
use crate::mcp::plugin_invoker::invoke_plugin_resource_sender;
use crate::mcp::plugin_invoker::invoke_plugin_tool_sender;
use crate::mcp::resource_reader::read_mcp_resource;
use crate::mcp_response_tracker::McpResponseTracker;
use smearor_mcp_server::McpCommand;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::box_payload;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use smearor_swipe_launcher_plugin_api::generate_type_id;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

pub async fn process_mcp_command(host: LauncherHost, command: McpCommand) {
    debug!(
        "process_mcp_command: ServiceManager ptr={:p} count={}",
        host.service_manager.as_ref(),
        host.service_manager.services.len()
    );
    match command {
        McpCommand::OpenArea { area_id, response } => {
            let result = with_first_area_manager(&host, |area_manager| area_manager.ensure_area(&area_id).map(|_| format!("Area {} opened", area_id)));
            let _ = response.send(result);
        }
        McpCommand::OpenTransientArea {
            area_id,
            source_area_id,
            response,
        } => {
            let result = with_first_area_manager(&host, |area_manager| {
                let area_config = area_manager
                    .config()
                    .get_area_config(&area_id)
                    .ok_or_else(|| format!("Area {} not found in config", area_id))?
                    .clone();
                let sender_id = area_manager.find_sender_id_for_transient(source_area_id.as_deref());
                area_manager
                    .add_transient_area(&area_id, area_config, sender_id.as_deref())
                    .map_err(|e| format!("Failed to open transient area {}: {}", area_id, e))?;
                Ok(format!("Transient area {} opened", area_id))
            });
            let _ = response.send(result);
        }
        McpCommand::CloseArea { area_id, response } => {
            let result = with_first_area_manager(&host, |area_manager| {
                area_manager
                    .remove_area(&area_id)
                    .map_err(|e| format!("Failed to close area {}: {}", area_id, e))?;
                Ok(format!("Area {} closed", area_id))
            });
            let _ = response.send(result);
        }
        McpCommand::FocusArea { area_id, response } => {
            let result = with_first_area_manager(&host, |area_manager| area_manager.focus(&area_id).map(|_| format!("Area {} focused", area_id)));
            let _ = response.send(result);
        }
        McpCommand::ListAreas { response } => {
            let result = with_first_area_manager(&host, |area_manager| {
                let areas = area_manager.list_areas();
                serde_json::to_string(&areas).map_err(|e| e.to_string())
            });
            let _ = response.send(result);
        }
        McpCommand::ListAllAreas { response } => {
            let result = with_first_area_manager(&host, |area_manager| {
                let areas = area_manager.list_all_areas();
                serde_json::to_string(&areas).map_err(|e| e.to_string())
            });
            let _ = response.send(result);
        }
        McpCommand::SendMessage {
            topic,
            payload,
            target_instance_id,
            response,
        } => {
            let result = send_mcp_message(&host, topic, payload, target_instance_id);
            let _ = response.send(result);
        }
        McpCommand::SendMultipleMessages { messages, response } => {
            let mut seen: Vec<(String, String)> = Vec::new();
            let mut sent_count: u32 = 0;
            let mut skipped_count: u32 = 0;
            for (topic, payload, target_instance_id) in messages {
                let payload_key = payload.to_string();
                if seen.iter().any(|(t, p)| t == &topic && p == &payload_key) {
                    skipped_count += 1;
                    continue;
                }
                seen.push((topic.clone(), payload_key));
                let result = send_mcp_message(&host, topic, payload, target_instance_id);
                if result.is_ok() {
                    sent_count += 1;
                }
            }
            let result = Ok(format!("{} messages sent, {} duplicates skipped", sent_count, skipped_count));
            let _ = response.send(result);
        }
        McpCommand::ReadResource { uri, response } => {
            let result = read_mcp_resource(&host, uri);
            let _ = response.send(result);
        }
        McpCommand::ToggleArea { area_id, response } => {
            let result = with_first_area_manager(&host, |area_manager| area_manager.toggle(&area_id).map(|_| format!("Area {} toggled", area_id)));
            let _ = response.send(result);
        }
        McpCommand::GetAreaConfig { area_id, response } => {
            let result = with_first_area_manager(&host, |area_manager| {
                let config = area_manager.get_area_config(&area_id)?;
                let mut config_value = serde_json::to_value(&config).map_err(|e| e.to_string())?;
                if let Some(plugins) = config_value.get_mut("plugins").and_then(|v| v.as_array_mut()) {
                    for plugin_value in plugins.iter_mut() {
                        if let Some(plugin_id) = plugin_value.get("id").and_then(|v| v.as_str()) {
                            if let Some(plugin_config) = area_manager.config().get_plugin_config(plugin_id) {
                                if let Some(plugin_object) = plugin_value.as_object_mut() {
                                    plugin_object.insert("config".to_string(), plugin_config.clone());
                                }
                            }
                        }
                    }
                }
                serde_json::to_string(&config_value).map_err(|e| e.to_string())
            });
            let _ = response.send(result);
        }
        McpCommand::LoadInstance {
            instance_id,
            config_path,
            instance_type,
            persist,
            response,
        } => {
            let parsed_type = match instance_type.as_str() {
                "headless" => crate::instance::InstanceType::Headless,
                "web" => crate::instance::InstanceType::Web,
                _ => crate::instance::InstanceType::Gtk,
            };
            let result = host.load_instance(instance_id, &config_path, parsed_type, persist, true);
            let _ = response.send(result);
        }
        McpCommand::StopInstance { instance_id, response } => {
            let result = host.stop_instance(&instance_id);
            let _ = response.send(result);
        }
        McpCommand::StartInstance { instance_id, response } => {
            let result = host.start_instance(&instance_id);
            let _ = response.send(result);
        }
        McpCommand::UnloadInstance { instance_id, response } => {
            let result = host.unload_instance(&instance_id);
            let _ = response.send(result);
        }
        McpCommand::ReloadInstance {
            instance_id,
            config_path,
            response,
        } => {
            let result = if config_path.is_empty() {
                let path = {
                    let instances = host.instances.lock();
                    match instances {
                        Ok(instances) => match instances.get(&instance_id) {
                            Some(instance) => instance.config_path.lock().ok().and_then(|g| g.clone()).unwrap_or_default(),
                            None => {
                                let _ = response.send(Err(format!("Instance '{}' not found", instance_id)));
                                return;
                            }
                        },
                        Err(e) => {
                            let _ = response.send(Err(format!("Failed to lock instances: {}", e)));
                            return;
                        }
                    }
                };
                if path.is_empty() {
                    Err(format!("No config path stored for instance '{}'", instance_id))
                } else {
                    host.reload_instance(&instance_id, &path)
                }
            } else {
                host.reload_instance(&instance_id, &config_path)
            };
            let _ = response.send(result);
        }
        McpCommand::ListInstances { response } => {
            let result = host.list_instances();
            let _ = response.send(result);
        }
        McpCommand::WebServerStatus { response } => {
            let result = host.web_server_status();
            let _ = response.send(result);
        }
        _ => {
            debug!("process_mcp_command received plugin command, ignoring (handled by process_plugin_command)");
        }
    }
}

/// Process plugin tool/resource invocations on a tokio task so they don't
/// block the GLib main context. Only `Send` types are used here.
pub async fn process_plugin_command(broker_sender: UnboundedSender<FfiEnvelope>, response_tracker: McpResponseTracker, command: McpCommand) {
    match command {
        McpCommand::InvokePluginTool {
            name,
            plugin_id: _,
            correlation_id,
            arguments,
            response,
        } => {
            let receiver = response_tracker.register(correlation_id.clone());
            let send_result = invoke_plugin_tool_sender(&broker_sender, &name, &correlation_id, &arguments);
            if send_result.is_ok() {
                let response_result = match tokio::time::timeout(tokio::time::Duration::from_secs(10), receiver).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Err("Plugin tool invocation dropped".to_string()),
                    Err(_) => Err("Plugin tool invocation timed out".to_string()),
                };
                let _ = response.send(response_result);
            } else {
                let _ = response.send(send_result.map(|_| String::new()).map_err(|e| e.to_string()));
            }
        }
        McpCommand::InvokePluginResource {
            uri,
            plugin_id: _,
            correlation_id,
            response,
        } => {
            let receiver = response_tracker.register(correlation_id.clone());
            let send_result = invoke_plugin_resource_sender(&broker_sender, &uri, &correlation_id);
            if send_result.is_ok() {
                let response_result = match tokio::time::timeout(tokio::time::Duration::from_secs(10), receiver).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Err("Plugin resource read dropped".to_string()),
                    Err(_) => Err("Plugin resource read timed out".to_string()),
                };
                let _ = response.send(response_result);
            } else {
                let _ = response.send(send_result.map(|_| String::new()).map_err(|e| e.to_string()));
            }
        }
        McpCommand::InvokePluginPrompt {
            name,
            plugin_id: _,
            correlation_id,
            arguments,
            response,
        } => {
            let receiver = response_tracker.register(correlation_id.clone());
            let send_result = invoke_plugin_prompt_sender(&broker_sender, &name, &correlation_id, &arguments);
            if send_result.is_ok() {
                let response_result = match tokio::time::timeout(tokio::time::Duration::from_secs(10), receiver).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Err("Plugin prompt invocation dropped".to_string()),
                    Err(_) => Err("Plugin prompt invocation timed out".to_string()),
                };
                let _ = response.send(response_result);
            } else {
                let _ = response.send(send_result.map(|_| String::new()).map_err(|e| e.to_string()));
            }
        }
        _ => {
            debug!("process_plugin_command received non-plugin command, ignoring");
        }
    }
}

fn with_first_area_manager<F, T>(host: &LauncherHost, callback: F) -> Result<T, String>
where
    F: FnOnce(&InstanceAreaManager) -> Result<T, String>,
{
    let instances = host.instances.lock().map_err(|_| "Failed to lock instances")?;
    let first_instance = instances.values().next().ok_or("No launcher instance available")?;
    let area_manager = first_instance.area_manager.lock().map_err(|_| "Failed to lock area manager")?;
    callback(&area_manager)
}

fn send_mcp_message(host: &LauncherHost, topic: String, payload: serde_json::Value, target_instance_id: Option<String>) -> Result<String, String> {
    let payload_json = payload.to_string();
    let payload_ptr = box_payload(payload_json);
    let envelope = FfiEnvelope::builder()
        .sender_id("mcp-server")
        .target_instance_id(target_instance_id.unwrap_or_default())
        .topic(topic)
        .type_id(generate_type_id("std::string::String"))
        .payload(payload_ptr)
        .destroy_payload(Some(default_destroy_payload))
        .clone_payload(Some(default_clone_payload::<String>))
        .build();
    host.broker_sender.send(envelope).map_err(|e| format!("Failed to send message: {}", e))?;
    Ok("Message sent".to_string())
}
