use crate::host::LauncherHost;
use smearor_mcp_server::McpCommand;

/// Handle instance lifecycle commands (load, start, stop, unload, reload, list).
pub(crate) fn handle_instance_command(host: &LauncherHost, command: McpCommand) {
    match command {
        McpCommand::LoadInstance(cmd) => {
            let parsed_type = match cmd.params.instance_type.as_str() {
                "headless" => crate::instance::InstanceType::Headless,
                "web" => crate::instance::InstanceType::Web,
                _ => crate::instance::InstanceType::Gtk,
            };
            let result = host.load_instance(cmd.params.instance_id, &cmd.params.config_path, parsed_type, cmd.params.persist, true);
            let _ = cmd.response.send(result);
        }
        McpCommand::StopInstance(cmd) => {
            let result = host.stop_instance(&cmd.params.instance_id);
            let _ = cmd.response.send(result);
        }
        McpCommand::StartInstance(cmd) => {
            let result = host.start_instance(&cmd.params.instance_id);
            let _ = cmd.response.send(result);
        }
        McpCommand::UnloadInstance(cmd) => {
            let result = host.unload_instance(&cmd.params.instance_id);
            let _ = cmd.response.send(result);
        }
        McpCommand::ReloadInstance(cmd) => {
            let config_path = cmd.params.config_path.unwrap_or_default();
            let result = if config_path.is_empty() {
                let path = {
                    let instances = host.instances.lock();
                    match instances {
                        Ok(instances) => match instances.get(&cmd.params.instance_id) {
                            Some(instance) => instance.config_path.lock().ok().and_then(|g| g.clone()).unwrap_or_default(),
                            None => {
                                let _ = cmd.response.send(Err(format!("Instance '{}' not found", cmd.params.instance_id)));
                                return;
                            }
                        },
                        Err(e) => {
                            let _ = cmd.response.send(Err(format!("Failed to lock instances: {}", e)));
                            return;
                        }
                    }
                };
                if path.is_empty() {
                    Err(format!("No config path stored for instance '{}'", cmd.params.instance_id))
                } else {
                    host.reload_instance(&cmd.params.instance_id, &path)
                }
            } else {
                host.reload_instance(&cmd.params.instance_id, &config_path)
            };
            let _ = cmd.response.send(result);
        }
        McpCommand::ListInstances(cmd) => {
            let result = host.list_instances();
            let _ = cmd.response.send(result);
        }
        _ => unreachable!("handle_instance_command received non-instance command"),
    }
}
