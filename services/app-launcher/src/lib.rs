use crate::service::AppLauncherService;
use abi_stable::RRef;
use abi_stable::std_types::RResult;
use abi_stable::std_types::RString;
use dashmap::DashMap;
use freedesktop_entry_parser::Entry;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use serde_json::Value;
use smearor_app_launcher_model::DesktopFileCommandMessage;
use smearor_app_launcher_model::TOPIC_COMMAND;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::LoadedService;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::ServiceVTable;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use tracing::Level;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::FmtSubscriber;

pub(crate) mod service;

// struct DesktopEntry {
//     exec: String,
// }
//
// impl DesktopEntry {
//     fn parse(path: &str) -> Option<Self> {
//         let content = std::fs::read_to_string(path).ok()?;
//         let mut exec = None;
//         let mut in_desktop_entry = false;
//
//         for line in content.lines() {
//             let line = line.trim();
//             if line.starts_with('[') && line.ends_with(']') {
//                 in_desktop_entry = line == "[Desktop Entry]";
//                 continue;
//             }
//             if !in_desktop_entry {
//                 continue;
//             }
//             if let Some((key, val)) = line.split_once('=') {
//                 if key.trim() == "Exec" {
//                     exec = Some(val.trim().to_string());
//                     break;
//                 }
//             }
//         }
//
//         Some(DesktopEntry {
//             exec: exec.unwrap_or_default(),
//         })
//     }
// }

unsafe extern "C" fn destroy_service(service: *mut ()) {
    if !service.is_null() {
        unsafe {
            let _ = Box::from_raw(service as *mut AppLauncherService);
        }
    }
}

unsafe extern "C" fn get_id(service: *mut ()) -> RString {
    if service.is_null() {
        return RString::from("");
    }
    let s = unsafe { &*(service as *const AppLauncherService) };
    RString::from(s.meta.id.clone())
}

unsafe extern "C" fn get_display_name(service: *mut ()) -> RString {
    if service.is_null() {
        return RString::from("");
    }
    let s = unsafe { &*(service as *const AppLauncherService) };
    RString::from(s.meta.display_name.clone())
}

unsafe extern "C" fn on_message(service: *mut (), message: FfiEnvelope) {
    debug!("AppLauncherService: Received message");
    if service.is_null() {
        debug!("service null");
        return;
    }
    if message.topic != TOPIC_COMMAND {
        debug!("topic {} != {TOPIC_COMMAND}", message.topic);
        return;
    }
    debug!("Getting service");
    let service = unsafe { &*(service as *const AppLauncherService) };
    debug!("Handle envelope message {}", message);
    service.handle_envelope_message(message);

    // let topic = message.topic.to_string();
    // let payload = message.payload.to_string();
    //
    // debug!("AppLauncher Service: Received message on topic '{}'", topic);
    //
    // if topic == "service/app_launcher/command" {
    //     if let Ok(parsed) = serde_json::from_str::<Value>(&payload) {
    //         let action = parsed.get("action").and_then(|v| v.as_str()).unwrap_or_default();
    //         let desktop_file = parsed.get("desktop_file").and_then(|v| v.as_str()).unwrap_or_default();
    //
    //         if desktop_file.is_empty() {
    //             error!("AppLauncher Service: Received command with empty desktop_file");
    //             return;
    //         }
    //
    //         match action {
    //             "Launch" => {
    //                 info!("AppLauncher Service: Launching app: {}", desktop_file);
    //                 let Ok(entry) = Entry::parse(desktop_file) else {
    //                     error!("AppLauncher Service: Failed to parse desktop file: {}", desktop_file);
    //                     return;
    //                 };
    //                 let Some(exec) = entry.get("Service", "Exec") else {
    //                     error!("Failed to get exec attr");
    //                     return;
    //                 };
    //
    //                 if let Some(entry) = DesktopEntry::parse(desktop_file) {
    //                     if !entry.exec.is_empty() {
    //                         let cmd_str = entry.exec;
    //                         let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    //
    //                         if let Some(program) = parts.first() {
    //                             let raw_args = &parts[1..];
    //                             // Sanitize placeholders like %u, %F
    //                             let clean_args: Vec<&str> = raw_args
    //                                 .iter()
    //                                 .map(|&arg| arg.trim())
    //                                 .filter(|&arg| !arg.is_empty() && !arg.starts_with('%'))
    //                                 .collect();
    //
    //                             let child = Command::new(program)
    //                                 .args(&clean_args)
    //                                 .stdin(Stdio::null())
    //                                 .stdout(Stdio::null())
    //                                 .stderr(Stdio::null())
    //                                 .spawn();
    //
    //                             match child {
    //                                 Ok(c) => {
    //                                     let pid = c.id();
    //                                     info!("AppLauncher Service: Successfully spawned {} with PID {}", program, pid);
    //                                     app_launcher_service.tracked_processes.entry(desktop_file.to_string()).or_default().push(pid);
    //                                     broadcast_status(&app_launcher_service.core_context, desktop_file, "Running");
    //                                 }
    //                                 Err(e) => {
    //                                     error!("AppLauncher Service: Failed to spawn Command {}: {}", program, e);
    //                                 }
    //                             }
    //                         }
    //                     }
    //                 } else {
    //                     error!("AppLauncher Service: Desktop app info could not be resolved for path: {}", desktop_file);
    //                 }
    //             }
    //             "Terminate" => {
    //                 info!("AppLauncher Service: Terminating app: {}", desktop_file);
    //
    //                 if let Some(mut r) = app_launcher_service.tracked_processes.get_mut(desktop_file) {
    //                     let pids = r.value_mut();
    //                     for &pid in pids.iter() {
    //                         let proc_path = format!("/proc/{}", pid);
    //                         if std::path::Path::new(&proc_path).exists() {
    //                             info!("AppLauncher Service: Sending SIGTERM to process {}", pid);
    //                             let posix_pid = Pid::from_raw(pid as i32);
    //                             if let Err(e) = kill(posix_pid, Signal::SIGTERM) {
    //                                 error!("AppLauncher Service: Failed to kill process {}: {}", pid, e);
    //                             }
    //                         }
    //                     }
    //                     pids.clear();
    //                 }
    //
    //                 app_launcher_service.tracked_processes.remove(desktop_file);
    //                 broadcast_status(&app_launcher_service.core_context, desktop_file, "Stopped");
    //             }
    //             _ => {
    //                 error!("AppLauncher Service: Unknown action '{}'", action);
    //             }
    //         }
    //     }
    // }
}

static VTABLE: ServiceVTable = ServiceVTable {
    destroy: destroy_service,
    get_id,
    get_display_name,
    on_message,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn smearor_service_create(
    config_json: *const i8,
    config_len: usize,
    core_context: FfiCoreContext,
) -> RResult<LoadedService, PluginConstructionError> {
    if config_json.is_null() {
        return RResult::RErr(PluginConstructionError::ConfigJsonIsNull);
    }

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::DEBUG.into()))
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    let slice = unsafe { std::slice::from_raw_parts(config_json as *const u8, config_len) };
    let config_str = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(e) => return RResult::RErr(PluginConstructionError::InvalidUtf8Config(e.to_string().into())),
    };
    debug!("AppLauncherServicesmearor_service_create: {config_str}");

    let config_value: Value = match serde_json::from_str(config_str) {
        Ok(v) => v,
        Err(e) => return RResult::RErr(PluginConstructionError::FailedToParseConfig(e.to_string().into())),
    };

    let config = PluginConfig { config: config_value };

    let core_context = if core_context.core_obj.is_null() { None } else { Some(core_context) };

    let app_launcher_service = match AppLauncherService::new(config, core_context) {
        Ok(app_launcher_service) => app_launcher_service,
        Err(e) => {
            return RResult::RErr(e);
        }
    };
    let service_box = Box::new(app_launcher_service);
    let service_instance = Box::into_raw(service_box) as *mut ();

    RResult::ROk(LoadedService {
        service_instance,
        vtable: RRef::new(&VTABLE),
    })
}
