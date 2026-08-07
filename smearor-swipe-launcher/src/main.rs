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
mod mcp;
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
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
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

        // Load instance via the unified lifecycle path.
        // persist=true so config-file instances survive restarts.
        // auto_start is controlled by the config file's `auto_start` field.
        if let Err(e) = host.load_instance(instance_id.clone(), &config_path.to_string_lossy(), instance_type, true, true) {
            error!("Failed to load instance '{}': {}", instance_id, e);
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
                    crate::mcp::process_plugin_command(broker_sender, response_tracker, command).await;
                });
            } else {
                crate::mcp::process_mcp_command(host_clone.clone(), command).await;
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
