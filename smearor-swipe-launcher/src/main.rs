mod area;
mod args;
mod config;
mod context;
mod css;
mod display;
mod error;
mod host;
mod init_tracing;
mod instance;
mod library_path;
mod mcp;
mod messages;
mod plugin;
mod service;
mod web;
mod window;

pub use args::launcher::SwipeLauncherArguments;
pub use config::launcher::SwipeLauncherConfig;
pub use host::LauncherHost;
pub use plugin::LoadedPlugin;
pub use plugin::PluginManager;
pub use service::LoadedService;
pub use service::ServiceManager;

use crate::config::discovery::bootstrap_configs;
use crate::config::services::ServicesConfig;
use crate::config::watcher::ConfigReloadRequest;
use crate::instance::get_instances_state_path;
use crate::instance::read_instances_state;
use crate::mcp::process_mcp_command;
use crate::mcp::process_plugin_command;
use crate::mcp::start_mcp_server;
use crate::web::WebServerConfig;
use clap::Parser;
use gtk4::Application;
use gtk4::glib::ControlFlow;
use gtk4::glib::MainContext;
use gtk4::prelude::ApplicationExt;
use miette::Result;
use smearor_mcp_server::McpCommand;
use smearor_mcp_server::McpServer;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tracing::debug;
use tracing::error;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = SwipeLauncherArguments::parse();

    let services_config = args.load_services_config()?;

    let log_buffer = crate::init_tracing::init(services_config.mcp.log_buffer_enabled, services_config.mcp.log_buffer_capacity)?;

    let config_paths = bootstrap_configs(&args)?;

    let gtk_app = Application::builder().application_id("com.smearor.swipe-launcher").build();
    let host = LauncherHost::new(gtk_app.clone(), log_buffer.clone());

    setup_host(&host, &args, &config_paths, &services_config)?;

    let mcp_handles = start_mcp_server(&host, &services_config, log_buffer);
    let reload_rx = start_infrastructure(&host, &services_config)?;

    spawn_main_loop_tasks(&MainContext::default(), &host, mcp_handles.command_receiver, reload_rx, &gtk_app);

    host.run();

    shutdown(host, mcp_handles.server).await;

    std::process::exit(0);
}

/// Load services, create instances, register JSON converters, and load persisted state.
fn setup_host(host: &LauncherHost, args: &SwipeLauncherArguments, config_paths: &[PathBuf], services_config: &ServicesConfig) -> Result<()> {
    host.load_services(services_config);
    if let Ok(mut guard) = host.services_config.lock() {
        *guard = Some(services_config.clone());
    }

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

        if let Err(e) = host.load_instance(instance_id.clone(), &config_path.to_string_lossy(), instance_type, true, true) {
            error!("Failed to load instance '{}': {}", instance_id, e);
        }
    }

    smearor_model_instance_control::register_json_converters(None);
    smearor_model_macropad::register_json_converters(None);
    smearor_model_widget::register_json_converters(None);

    host.load_persisted_instances();

    let state_path = get_instances_state_path();
    for entry in read_instances_state(&state_path) {
        let persisted_config_path = PathBuf::from(&entry.config_path);
        if !config_paths.iter().any(|p| *p == persisted_config_path) {
            if let Ok(config) = args.load_config_from_file(&persisted_config_path) {
                let include_paths = config.collect_include_paths(&persisted_config_path);
                host.config_watcher.add_config(&persisted_config_path, &entry.instance_id, &include_paths);
            }
        }
    }

    debug!("Application initialized successfully");
    Ok(())
}

/// Start the web server, CSS watcher, build the UI, and start the config file watcher.
fn start_infrastructure(host: &LauncherHost, services_config: &ServicesConfig) -> Result<async_channel::Receiver<ConfigReloadRequest>> {
    if services_config.web.enabled {
        let web_config = WebServerConfig::builder()
            .port(services_config.web.port)
            .enabled(true)
            .bind_address(services_config.web.bind_address.clone())
            .auth_token(services_config.web.auth_token.clone())
            .allowed_origins(services_config.web.allowed_origins.clone())
            .build();
        host.start_web_server(web_config);
        debug!("Web server started on {}:{}", services_config.web.bind_address, services_config.web.port);
    }

    host.css_watcher.watch_global_css();
    host.css_watcher.start();

    host.build_ui()?;

    Ok(host.config_watcher.start())
}

/// Spawn the MCP command loop, config reload loop, and SIGINT handler on the GLib main context.
fn spawn_main_loop_tasks(
    main_context: &MainContext,
    host: &LauncherHost,
    mcp_receiver: async_channel::Receiver<McpCommand>,
    reload_rx: async_channel::Receiver<ConfigReloadRequest>,
    gtk_app: &Application,
) {
    let host_clone = host.clone();
    main_context.spawn_local(async move {
        while let Ok(command) = mcp_receiver.recv().await {
            if matches!(
                command,
                McpCommand::InvokePluginTool(..) | McpCommand::InvokePluginResource(..) | McpCommand::InvokePluginPrompt(..)
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
}

/// Stop servers, remove areas, unload services, and shut down watchers.
async fn shutdown(host: LauncherHost, mcp_server: Option<McpServer>) {
    if let Some(mut server) = mcp_server {
        server.stop();
    }

    if let Ok(instances) = host.instances.lock() {
        for instance in instances.values() {
            if let Ok(area_manager) = instance.area_manager.lock() {
                area_manager.remove_all_areas_immediate();
            }
        }
    }

    host.service_manager.unload_services();
    host.css_watcher.shutdown();
    host.config_watcher.shutdown();

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
}
