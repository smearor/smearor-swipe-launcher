use abi_stable::RRef;
use abi_stable::std_types::RResult;
use abi_stable::std_types::RString;
use dashmap::DashMap;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use smearor_plugin_api::FfiCoreContext;
use smearor_plugin_api::FfiEnvelope;
use smearor_plugin_api::LoadedService;
use smearor_plugin_api::ServiceVTable;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use tracing::debug;
use tracing::error;
use tracing::info;

struct DesktopEntry {
    exec: String,
}

impl DesktopEntry {
    fn parse(path: &str) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let mut exec = None;
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                in_desktop_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_desktop_entry {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                if key.trim() == "Exec" {
                    exec = Some(val.trim().to_string());
                    break;
                }
            }
        }

        Some(DesktopEntry {
            exec: exec.unwrap_or_default(),
        })
    }
}

pub struct AppLauncherService {
    pub id: String,
    pub display_name: String,
    pub tracked_processes: Arc<DashMap<String, Vec<u32>>>,
    pub core_context: Option<FfiCoreContext>,
}

impl AppLauncherService {
    pub fn new(core_context: Option<FfiCoreContext>) -> Self {
        let service = AppLauncherService {
            id: "app_launcher".to_string(),
            display_name: "Application Launcher Service".to_string(),
            tracked_processes: Arc::new(DashMap::new()),
            core_context,
        };

        // Start local reaper timer loop on the GLib main context
        let tracked_processes_clone = service.tracked_processes.clone();
        let core_context_clone = service.core_context.clone();
        gtk4::glib::timeout_add_seconds_local(2, move || {
            let mut completed_apps = Vec::new();

            tracked_processes_clone.retain(|desktop_file, pids| {
                // Retain only currently active processes via procfs
                pids.retain(|&pid| {
                    let proc_path = format!("/proc/{}", pid);
                    std::path::Path::new(&proc_path).exists()
                });

                if pids.is_empty() {
                    completed_apps.push(desktop_file.clone());
                    false // Remove from DashMap
                } else {
                    true // Keep in DashMap
                }
            });

            for desktop_file in completed_apps {
                info!("AppLauncher Service: App exited naturally: {}", desktop_file);
                broadcast_status(&core_context_clone, &desktop_file, "Stopped");
            }

            gtk4::glib::ControlFlow::Continue
        });

        service
    }
}

fn broadcast_status(context: &Option<FfiCoreContext>, desktop_file: &str, status: &str) {
    if let Some(ctx) = context {
        let envelope = FfiEnvelope {
            sender_id: RString::from("app_launcher"),
            topic: RString::from("service/app_launcher/status"),
            payload: RString::from(format!("{{\"desktop_file\": \"{}\", \"status\": \"{}\"}}", desktop_file, status)),
        };
        unsafe {
            (ctx.vtable.get().send_message)(ctx.core_obj, envelope);
        }
    }
}

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
    RString::from(s.id.clone())
}

unsafe extern "C" fn get_display_name(service: *mut ()) -> RString {
    if service.is_null() {
        return RString::from("");
    }
    let s = unsafe { &*(service as *const AppLauncherService) };
    RString::from(s.display_name.clone())
}

unsafe extern "C" fn on_message(service: *mut (), message: FfiEnvelope) {
    debug!("AppLauncherService: Received message");
    if service.is_null() {
        return;
    }

    let s = unsafe { &*(service as *const AppLauncherService) };
    let topic = message.topic.to_string();
    let payload = message.payload.to_string();

    debug!("AppLauncher Service: Received message on topic '{}'", topic);

    if topic == "service/app_launcher/command" {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&payload) {
            let action = parsed.get("action").and_then(|v| v.as_str()).unwrap_or_default();
            let desktop_file = parsed.get("desktop_file").and_then(|v| v.as_str()).unwrap_or_default();

            if desktop_file.is_empty() {
                error!("AppLauncher Service: Received command with empty desktop_file");
                return;
            }

            match action {
                "Launch" => {
                    info!("AppLauncher Service: Launching app: {}", desktop_file);

                    if let Some(entry) = DesktopEntry::parse(desktop_file) {
                        if !entry.exec.is_empty() {
                            let cmd_str = entry.exec;
                            let parts: Vec<&str> = cmd_str.split_whitespace().collect();

                            if let Some(program) = parts.first() {
                                let raw_args = &parts[1..];
                                // Sanitize placeholders like %u, %F
                                let clean_args: Vec<&str> = raw_args
                                    .iter()
                                    .map(|&arg| arg.trim())
                                    .filter(|&arg| !arg.is_empty() && !arg.starts_with('%'))
                                    .collect();

                                let child = Command::new(program)
                                    .args(&clean_args)
                                    .stdin(Stdio::null())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .spawn();

                                match child {
                                    Ok(c) => {
                                        let pid = c.id();
                                        info!("AppLauncher Service: Successfully spawned {} with PID {}", program, pid);
                                        s.tracked_processes.entry(desktop_file.to_string()).or_default().push(pid);
                                        broadcast_status(&s.core_context, desktop_file, "Running");
                                    }
                                    Err(e) => {
                                        error!("AppLauncher Service: Failed to spawn Command {}: {}", program, e);
                                    }
                                }
                            }
                        }
                    } else {
                        error!("AppLauncher Service: Desktop app info could not be resolved for path: {}", desktop_file);
                    }
                }
                "Terminate" => {
                    info!("AppLauncher Service: Terminating app: {}", desktop_file);

                    if let Some(mut r) = s.tracked_processes.get_mut(desktop_file) {
                        let pids = r.value_mut();
                        for &pid in pids.iter() {
                            let proc_path = format!("/proc/{}", pid);
                            if std::path::Path::new(&proc_path).exists() {
                                info!("AppLauncher Service: Sending SIGTERM to process {}", pid);
                                let posix_pid = Pid::from_raw(pid as i32);
                                if let Err(e) = kill(posix_pid, Signal::SIGTERM) {
                                    error!("AppLauncher Service: Failed to kill process {}: {}", pid, e);
                                }
                            }
                        }
                        pids.clear();
                    }

                    s.tracked_processes.remove(desktop_file);
                    broadcast_status(&s.core_context, desktop_file, "Stopped");
                }
                _ => {
                    error!("AppLauncher Service: Unknown action '{}'", action);
                }
            }
        }
    }
}

static VTABLE: ServiceVTable = ServiceVTable {
    destroy: destroy_service,
    get_id,
    get_display_name,
    on_message,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn smearor_service_create(config_json: *const i8, config_len: usize, core_context: FfiCoreContext) -> RResult<LoadedService, RString> {
    let _config_json = config_json;
    let _config_len = config_len;
    let core_context = if core_context.core_obj.is_null() { None } else { Some(core_context) };

    let service = AppLauncherService::new(core_context);
    let service_box = Box::new(service);
    let service_instance = Box::into_raw(service_box) as *mut ();

    RResult::ROk(LoadedService {
        service_instance,
        vtable: RRef::new(&VTABLE),
    })
}
