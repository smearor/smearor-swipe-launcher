mod application;
mod area;
mod args;
mod config;
mod context;
mod css;
mod display;
mod error;
mod instance;
mod json_converter;
mod library_path;
mod mcp_registry;
mod mcp_response_tracker;
mod messages;
mod plugin;
mod plugin_manager;
mod service;
mod service_manager;
mod web;
mod window;

pub use application::LauncherHost;
pub use args::launcher::SwipeLauncherArguments;
pub use config::launcher::SwipeLauncherConfig;
pub use plugin::LoadedPlugin;
pub use plugin_manager::PluginManager;
pub use service::LoadedService;
pub use service_manager::ServiceManager;

use crate::mcp_response_tracker::McpResponseTracker;
use clap::Parser;
use gtk4::Application;
use gtk4::glib::ControlFlow;
use gtk4::glib::MainContext;
use gtk4::prelude::ApplicationExt;
use miette::IntoDiagnostic;
use miette::Result;
use smearor_mcp_server::McpCommand;
use smearor_mcp_server::McpServer;
use smearor_mcp_server::McpServerConfig;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tracing::error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = SwipeLauncherArguments::parse();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
    tracing_log::LogTracer::init().into_diagnostic()?;

    // Bootstrap user configs from system defaults on first run
    let discovery_service = crate::config::discovery::ConfigDiscoveryService::new();
    discovery_service.bootstrap_user_configs();

    // Discover launcher config files (CLI > working dir > XDG config dir > system default)
    let config_paths = discovery_service.discover_launcher_configs(&args.config)?;
    if config_paths.is_empty() {
        return Err(miette::miette!(
            "No launcher configuration files found. \
            Specify via --config, or place *.toml files in the working directory or ~/.config/smearor/launcher/"
        ));
    }
    discovery_service.validate_config_paths(&config_paths)?;
    debug!("Starting smearor-swipe-launcher with config files: {:?}", config_paths);

    let gtk_app = Application::builder().application_id("com.smearor.swipe-launcher").build();

    let host = LauncherHost::new(gtk_app.clone());

    // Load shared services from dedicated config
    let services_config = args.load_services_config()?;
    host.load_services(&services_config);
    if let Ok(mut guard) = host.services_config.lock() {
        *guard = Some(services_config.clone());
    }

    // Create one instance per config file
    for (index, config_path) in config_paths.iter().enumerate() {
        let config = args.load_config_from_file(config_path)?;
        let instance_id = args
            .instance_id
            .get(index)
            .cloned()
            .unwrap_or_else(|| config_path.file_stem().unwrap_or_default().to_string_lossy().to_string());
        let include_paths = config.collect_include_paths(config_path);
        host.config_watcher.add_config(config_path, &instance_id, &include_paths);
        let instance_type = config.launcher.instance_type.to_instance_type();
        host.create_instance(instance_id.clone(), config, instance_type);

        if instance_type == crate::instance::InstanceType::Gtk {
            host.css_watcher.watch_instance_css(config_path);
        }

        // For web and headless instances, build areas (no GTK window).
        if instance_type == crate::instance::InstanceType::Web || instance_type == crate::instance::InstanceType::Headless {
            if let Ok(instances) = host.instances.lock() {
                if let Some(instance) = instances.get(&instance_id) {
                    instance.build_headless();
                }
            }
        }
    }

    // Register JSON converters for instance-control messages (core.instance.load/stop/reload)
    smearor_model_instance_control::register_json_converters(None);

    // Register JSON converters for MacroPad messages (input/connection/command)
    smearor_model_macropad::register_json_converters(None);

    // Register JSON converters for widget update messages
    smearor_model_widget::register_json_converters(None);

    // Load any persisted dynamic instances from the state file
    host.load_persisted_instances();

    // Register persisted instance configs with the file watcher
    let state_path = crate::instance::get_instances_state_path();
    for entry in crate::instance::read_instances_state(&state_path) {
        let persisted_config_path = std::path::PathBuf::from(&entry.config_path);
        if !config_paths.iter().any(|p| *p == persisted_config_path) {
            if let Ok(config) = args.load_config_from_file(&persisted_config_path) {
                let include_paths = config.collect_include_paths(&persisted_config_path);
                host.config_watcher.add_config(&persisted_config_path, &entry.instance_id, &include_paths);
            }
        }
    }

    debug!("Application initialized successfully");

    let mcp_config = McpServerConfig {
        bind_address: services_config.mcp.bind_address.clone(),
        port: services_config.mcp.port,
        auth_token: services_config.mcp.auth_token.clone(),
    };
    let (mcp_command_sender, mcp_receiver) = async_channel::unbounded::<McpCommand>();
    let mut mcp_server = McpServer::new(mcp_config, host.mcp_registry.clone(), mcp_command_sender.clone());
    host.set_mcp_command_sender(mcp_command_sender);
    mcp_server.start();
    let _mcp_server = Some(mcp_server);

    // Start the embedded web server if enabled in services config
    if services_config.web.enabled {
        let web_config = crate::web::WebServerConfig {
            port: services_config.web.port,
            enabled: true,
            bind_address: services_config.web.bind_address.clone(),
            auth_token: services_config.web.auth_token.clone(),
            allowed_origins: services_config.web.allowed_origins.clone(),
        };
        host.start_web_server(web_config);
        debug!("Web server started on {}:{}", services_config.web.bind_address, services_config.web.port);
    }

    // Start CSS file watcher (global + per-instance hot-reload)
    host.css_watcher.watch_global_css();
    host.css_watcher.start();

    host.build_ui()?;

    let main_context = MainContext::default();
    let host_clone = host.clone();
    main_context.spawn_local(async move {
        while let Ok(command) = mcp_receiver.recv().await {
            if matches!(
                command,
                McpCommand::InvokePluginTool { .. } | McpCommand::InvokePluginResource { .. } | McpCommand::InvokePluginPrompt { .. }
            ) {
                let broker_sender = host_clone.broker_sender.clone();
                let response_tracker = host_clone.mcp_response_tracker.clone();
                tokio::spawn(async move {
                    process_plugin_command(broker_sender, response_tracker, command).await;
                });
            } else {
                process_mcp_command(host_clone.clone(), command).await;
            }
        }
    });

    // Start config file watcher and handle reload requests on the main context
    let reload_rx = host.config_watcher.start();
    let host_clone_for_reload = host.clone();
    main_context.spawn_local(async move {
        while let Ok(request) = reload_rx.recv().await {
            debug!("Reloading instance '{}' from config '{}'", request.instance_id, request.config_path.display());
            match host_clone_for_reload.reload_instance(&request.instance_id, &request.config_path.to_string_lossy()) {
                Ok(msg) => debug!("Config reload: {}", msg),
                Err(e) => error!("Config reload failed for instance '{}': {}", request.instance_id, e),
            }
        }
    });

    // Install SIGINT handler: Ctrl-C should quit the GTK main loop gracefully
    // so the cleanup code below (service teardown, MCP server stop) runs.
    // Without this, SIGINT kills the process immediately and service threads
    // (network polling, audio events, etc.) keep running in the background.
    //
    // gtk4::Application is not Send, so we use an Arc<AtomicBool> flag:
    // a tokio task waits for SIGINT and sets the flag; a GLib timeout source
    // polls the flag on the main thread and calls gtk_app.quit().
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_for_signal = shutdown_flag.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            debug!("SIGINT received, quitting GTK application gracefully");
            shutdown_flag_for_signal.store(true, Ordering::SeqCst);
        }
    });
    let gtk_app_for_signal = gtk_app.clone();
    gtk4::glib::source::timeout_add_local(std::time::Duration::from_millis(200), move || {
        if shutdown_flag.load(Ordering::SeqCst) {
            gtk_app_for_signal.quit();
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
    });

    host.run();

    // GTK has quit — explicitly clean up before process exit.
    // The GLib spawn_local task may still hold Arc<ServiceManager> references
    // that prevent Drop from running, and the Axum server task keeps tokio
    // worker threads alive. We stop the MCP server and unload services
    // explicitly, then force exit.
    if let Some(mut server) = _mcp_server {
        server.stop();
    }

    // Remove all areas synchronously (without animation) to ensure no
    // pending GLib timeout callbacks remain active during service teardown.
    if let Ok(instances) = host.instances.lock() {
        for instance in instances.values() {
            if let Ok(area_manager) = instance.area_manager.lock() {
                area_manager.remove_all_areas_immediate();
            }
        }
    }

    host.service_manager.unload_services();

    // Stop CSS file watchers and cancel debounce tasks.
    host.css_watcher.shutdown();

    // Stop config file watcher and cancel debounce task.
    host.config_watcher.shutdown();

    // Brief grace period to let pending GLib timeouts, async tasks, and
    // service Drop handlers fully drain before process exit.
    std::thread::sleep(std::time::Duration::from_millis(500));

    std::process::exit(0);
}

async fn process_mcp_command(host: LauncherHost, command: McpCommand) {
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
            response,
        } => {
            let parsed_type = match instance_type.as_str() {
                "headless" => crate::instance::InstanceType::Headless,
                "web" => crate::instance::InstanceType::Web,
                _ => crate::instance::InstanceType::Gtk,
            };
            let result = host.load_instance(instance_id, &config_path, parsed_type);
            let _ = response.send(result);
        }
        McpCommand::StopInstance { instance_id, response } => {
            let result = host.stop_instance(&instance_id);
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
async fn process_plugin_command(broker_sender: UnboundedSender<FfiEnvelope>, response_tracker: McpResponseTracker, command: McpCommand) {
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
                let _ = response.send(send_result.map(|_| String::new()));
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
                let _ = response.send(send_result.map(|_| String::new()));
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
                let _ = response.send(send_result.map(|_| String::new()));
            }
        }
        _ => {
            debug!("process_plugin_command received non-plugin command, ignoring");
        }
    }
}

fn with_first_area_manager<F, T>(host: &LauncherHost, callback: F) -> Result<T, String>
where
    F: FnOnce(&crate::area::instance_area_manager::InstanceAreaManager) -> Result<T, String>,
{
    let instances = host.instances.lock().map_err(|_| "Failed to lock instances")?;
    let first_instance = instances.values().next().ok_or("No launcher instance available")?;
    let area_manager = first_instance.area_manager.lock().map_err(|_| "Failed to lock area manager")?;
    callback(&area_manager)
}

fn send_mcp_message(host: &LauncherHost, topic: String, payload: serde_json::Value, target_instance_id: Option<String>) -> Result<String, String> {
    let payload_json = payload.to_string();
    let payload_ptr = Box::into_raw(Box::new(payload_json)) as *mut core::ffi::c_void;
    let envelope = smearor_swipe_launcher_plugin_api::FfiEnvelope {
        sender_id: stabby::string::String::from("mcp-server"),
        target_instance_id: stabby::string::String::from(target_instance_id.unwrap_or_default()),
        topic: stabby::string::String::from(topic),
        type_id: smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String"),
        payload: payload_ptr,
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<String>),
    };
    host.broker_sender.send(envelope).map_err(|e| format!("Failed to send message: {}", e))?;
    Ok("Message sent".to_string())
}

fn read_mcp_resource(host: &LauncherHost, uri: String) -> Result<String, String> {
    if uri == "area://list" {
        with_first_area_manager(host, |area_manager| {
            let areas = area_manager.list_areas();
            serde_json::to_string(&areas).map_err(|e| e.to_string())
        })
    } else if uri.starts_with("area://") && uri.ends_with("/state") {
        let area_id = uri.trim_start_matches("area://").trim_end_matches("/state");
        with_first_area_manager(host, |area_manager| {
            let areas = area_manager.list_areas();
            let area = areas.into_iter().find(|a| a.area_id == area_id).ok_or(format!("Area {} not found", area_id))?;
            serde_json::to_string(&area).map_err(|e| e.to_string())
        })
    } else if uri == "area://plugins" {
        with_first_area_manager(host, |area_manager| {
            let config = area_manager.config();
            let areas: Vec<serde_json::Value> = config
                .entries
                .iter()
                .filter_map(|(area_id, entry)| {
                    let area_config = match entry {
                        config::area::config_entry::ConfigEntry::Area(ac) => ac,
                        config::area::config_entry::ConfigEntry::Plugin(_) => return None,
                    };
                    let plugins: Vec<serde_json::Value> = area_config
                        .plugins
                        .iter()
                        .filter(|p| !p.disabled)
                        .map(|p| {
                            serde_json::json!({
                                "id": p.id,
                                "path": p.path,
                                "name": p.name,
                                "widget": p.widget,
                            })
                        })
                        .collect();
                    Some(serde_json::json!({
                        "area_id": area_id,
                        "plugins": plugins,
                    }))
                })
                .collect();
            serde_json::to_string(&serde_json::json!({ "areas": areas })).map_err(|e| e.to_string())
        })
    } else if uri == "area://buttons" {
        with_first_area_manager(host, |area_manager| {
            let config = area_manager.config();
            let mut buttons: Vec<serde_json::Value> = Vec::new();
            for (area_id, entry) in &config.entries {
                let area_config = match entry {
                    config::area::config_entry::ConfigEntry::Area(ac) => ac,
                    config::area::config_entry::ConfigEntry::Plugin(_) => continue,
                };
                for plugin in &area_config.plugins {
                    if plugin.path.as_deref().unwrap_or("").contains("libsmearor_button_widget") && !plugin.disabled {
                        if let Some(button_config) = config.get_plugin_config(&plugin.id) {
                            buttons.push(serde_json::json!({
                                "id": plugin.id,
                                "area_id": area_id,
                                "config": button_config,
                            }));
                        }
                    }
                }
            }
            serde_json::to_string(&serde_json::json!({ "buttons": buttons })).map_err(|e| e.to_string())
        })
    } else if uri == "plugin://list" {
        read_plugin_list(host)
    } else {
        Err(format!("Resource {} not implemented", uri))
    }
}

fn read_plugin_list(host: &LauncherHost) -> Result<String, String> {
    let mut plugins: Vec<serde_json::Value> = Vec::new();

    if let Ok(guard) = host.services_config.lock() {
        if let Some(services_config) = guard.as_ref() {
            for service in &services_config.services {
                plugins.push(serde_json::json!({
                    "id": service.id,
                    "path": service.path,
                    "name": service.name,
                    "type": "service",
                }));
            }
        }
    }

    with_first_area_manager(host, |area_manager| {
        let config = area_manager.config();
        for (_area_id, entry) in &config.entries {
            let area_config = match entry {
                config::area::config_entry::ConfigEntry::Area(ac) => ac,
                config::area::config_entry::ConfigEntry::Plugin(_) => continue,
            };
            for plugin in &area_config.plugins {
                if !plugin.disabled {
                    plugins.push(serde_json::json!({
                        "id": plugin.id,
                        "path": plugin.path,
                        "name": plugin.name,
                        "type": "widget",
                    }));
                }
            }
        }
        Ok(())
    })?;

    serde_json::to_string(&serde_json::json!({ "plugins": plugins })).map_err(|e| e.to_string())
}

fn invoke_plugin_tool_sender(
    broker_sender: &UnboundedSender<FfiEnvelope>,
    name: &str,
    correlation_id: &str,
    arguments: &serde_json::Value,
) -> Result<(), String> {
    debug!("invoke_plugin_tool_sender: name={} correlation_id={}", name, correlation_id);
    let message = InvokeToolMessage::new(name, correlation_id, &arguments.to_string());
    let payload_ptr = Box::into_raw(Box::new(message)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from("mcp-server"),
        target_instance_id: stabby::string::String::from("*"),
        topic: stabby::string::String::from(InvokeToolMessage::topic()),
        type_id: InvokeToolMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<InvokeToolMessage>),
    };
    broker_sender
        .send(envelope)
        .map_err(|e| format!("Failed to send plugin tool invocation: {}", e))?;
    Ok(())
}

fn invoke_plugin_resource_sender(broker_sender: &UnboundedSender<FfiEnvelope>, uri: &str, correlation_id: &str) -> Result<(), String> {
    debug!("invoke_plugin_resource_sender: uri={} correlation_id={}", uri, correlation_id);
    let message = InvokeResourceMessage::new(uri, correlation_id);
    let payload_ptr = Box::into_raw(Box::new(message)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from("mcp-server"),
        target_instance_id: stabby::string::String::from("*"),
        topic: stabby::string::String::from(InvokeResourceMessage::topic()),
        type_id: InvokeResourceMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<InvokeResourceMessage>),
    };
    broker_sender
        .send(envelope)
        .map_err(|e| format!("Failed to send plugin resource read: {}", e))?;
    Ok(())
}

fn invoke_plugin_prompt_sender(
    broker_sender: &UnboundedSender<FfiEnvelope>,
    name: &str,
    correlation_id: &str,
    arguments: &serde_json::Value,
) -> Result<(), String> {
    debug!("invoke_plugin_prompt_sender: name={} correlation_id={}", name, correlation_id);
    let message = InvokePromptMessage::new(name, correlation_id, &arguments.to_string());
    let payload_ptr = Box::into_raw(Box::new(message)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from("mcp-server"),
        target_instance_id: stabby::string::String::from("*"),
        topic: stabby::string::String::from(InvokePromptMessage::topic()),
        type_id: InvokePromptMessage::TYPE_ID,
        payload: payload_ptr,
        destroy_payload: Some(default_destroy_payload),
        clone_payload: Some(default_clone_payload::<InvokePromptMessage>),
    };
    broker_sender
        .send(envelope)
        .map_err(|e| format!("Failed to send plugin prompt invocation: {}", e))?;
    Ok(())
}
