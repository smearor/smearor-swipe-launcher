mod application;
mod area;
mod args;
mod config;
mod context;
mod css_provider;
mod display;
mod error;
mod instance;
mod json_converter;
mod mcp_registry;
mod mcp_response_tracker;
mod messages;
mod plugin;
mod plugin_manager;
mod service;
mod service_manager;
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
use gtk4::glib::MainContext;
use miette::IntoDiagnostic;
use miette::Result;
use smearor_mcp_server::McpCommand;
use smearor_mcp_server::McpServer;
use smearor_mcp_server::McpServerConfig;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

/// Destroy a typed payload that was allocated with `Box::into_raw`.
extern "C" fn destroy_payload<T>(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut T);
        }
    }
}

/// Clone a typed payload for safe `FfiEnvelope` cloning across multiple
/// broker recipients.
extern "C" fn clone_payload<T: Clone>(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let typed = ptr as *mut T;
        let cloned = (*typed).clone();
        Box::into_raw(Box::new(cloned)) as *mut core::ffi::c_void
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = SwipeLauncherArguments::parse();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;

    debug!("Starting smearor-swipe-launcher with config files: {:?}", args.config);

    let gtk_app = Application::builder().application_id("com.smearor.swipe-launcher").build();

    let host = LauncherHost::new(gtk_app.clone());

    // Load shared services from dedicated config
    let services_config = args.load_services_config()?;
    host.load_services(&services_config);

    // Create one instance per config file
    for (index, config_path) in args.config.iter().enumerate() {
        let config = args.load_config_from_file(config_path)?;
        let instance_id = args
            .instance_id
            .get(index)
            .cloned()
            .unwrap_or_else(|| config_path.file_stem().unwrap_or_default().to_string_lossy().to_string());
        host.create_instance(instance_id, config);
    }

    debug!("Application initialized successfully");

    let (mut mcp_server, mcp_receiver) = McpServer::new(McpServerConfig::default(), host.mcp_registry.clone());
    mcp_server.start();
    let _mcp_server = Some(mcp_server);

    host.build_ui()?;

    let main_context = MainContext::default();
    let host_clone = host.clone();
    main_context.spawn_local(async move {
        while let Ok(command) = mcp_receiver.recv().await {
            if matches!(command, McpCommand::InvokePluginTool { .. } | McpCommand::InvokePluginResource { .. }) {
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

    host.run();

    Ok(())
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
                    .config
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
                serde_json::to_string(&config).map_err(|e| e.to_string())
            });
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
        _ => {
            debug!("process_plugin_command received non-plugin command, ignoring");
        }
    }
}

fn with_first_area_manager<F, T>(host: &LauncherHost, callback: F) -> Result<T, String>
where
    F: FnOnce(&crate::area::area_manager::AreaManager) -> Result<T, String>,
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
        clone_payload: None,
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
    } else {
        Err(format!("Resource {} not implemented", uri))
    }
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
        destroy_payload: Some(destroy_payload::<InvokeToolMessage>),
        clone_payload: Some(clone_payload::<InvokeToolMessage>),
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
        destroy_payload: Some(destroy_payload::<InvokeResourceMessage>),
        clone_payload: Some(clone_payload::<InvokeResourceMessage>),
    };
    broker_sender
        .send(envelope)
        .map_err(|e| format!("Failed to send plugin resource read: {}", e))?;
    Ok(())
}
