use dashmap::DashMap;
use freedesktop_entry_parser::Entry;
use gtk4::glib::ControlFlow;
use gtk4::glib::timeout_add_seconds_local;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use smearor_app_launcher_model::DesktopFileCommandAction;
use smearor_app_launcher_model::DesktopFileCommandMessage;
use smearor_app_launcher_model::DesktopFileStatus;
use smearor_app_launcher_model::DesktopFileStatusMessage;
use smearor_app_launcher_model::TOPIC_COMMAND;
use smearor_app_launcher_model::TOPIC_STATUS;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;

pub struct AppLauncherService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub tracked_processes: Arc<DashMap<String, Vec<u32>>>,
}

impl AppLauncherService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionError> {
        let service = AppLauncherService {
            meta: PluginMeta::try_from(&config)?,
            core_context,
            tracked_processes: Arc::new(DashMap::new()),
        };

        // Start local reaper timer loop on the GLib main context
        service.init_reaper();

        Ok(service)
    }

    pub fn init_reaper(&self) {
        let broadcaster = self.get_broadcaster();
        let tracked_processes = self.tracked_processes.clone();
        timeout_add_seconds_local(2, move || {
            let mut completed_apps = Vec::new();
            tracked_processes.retain(|desktop_file, pids| {
                // Retain only currently active processes via procfs
                pids.retain(|&pid| {
                    let proc_path = format!("/proc/{}", pid);
                    Path::new(&proc_path).exists()
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
                broadcaster.broadcast_message(TOPIC_STATUS, DesktopFileStatusMessage::stopped(&desktop_file));
            }

            ControlFlow::Continue
        });
    }

    fn handle_exec(&self, desktop_file: &str) {
        info!("AppLauncher Service: Launching app: {desktop_file}");
        let entry = match Entry::parse_file(desktop_file) {
            Ok(entry) => entry,
            Err(e) => {
                error!("AppLauncher Service: Failed to parse desktop file {desktop_file}: {e}");
                return;
            }
        };
        let Some(exec) = entry.get("Desktop Entry", "Exec") else {
            error!("Failed to get exec attr");
            return;
        };
        trace!("Exec: {:?}", exec);
        if let Some(exec_first) = exec.first() {
            trace!("program: {exec_first}");
            let mut raw_args = exec_first.split(" ").into_iter().map(|arg| arg.to_string()).collect::<Vec<String>>();
            trace!("args: {:?}", raw_args);
            let Some(program) = raw_args.first().cloned() else {
                error!("Failed to get program attr");
                return;
            };
            trace!("raw_args: {:?}", raw_args);
            raw_args.remove(0);
            // Sanitize placeholders like %u, %F
            let clean_args: Vec<String> = raw_args
                .iter()
                .map(|arg| arg.trim().to_string())
                .filter(|arg| !arg.is_empty() && !arg.starts_with('%'))
                .collect();
            trace!("clean_args: {:?}", clean_args);

            let child = Command::new(program.clone())
                .args(&clean_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            match child {
                Ok(c) => {
                    let pid = c.id();
                    info!("AppLauncher Service: Successfully spawned {} with PID {}", program, pid);
                    self.tracked_processes.entry(desktop_file.to_string()).or_default().push(pid);
                    self.broadcast_message(TOPIC_STATUS, DesktopFileStatusMessage::running(desktop_file));
                }
                Err(e) => {
                    error!("AppLauncher Service: Failed to spawn Command {}: {}", program, e);
                }
            }
        }
    }

    fn handle_terminate(&self, desktop_file: &str) {
        info!("AppLauncher Service: Terminating app: {desktop_file}");
        if let Some(mut r) = self.tracked_processes.get_mut(desktop_file) {
            let pids = r.value_mut();
            for &pid in pids.iter() {
                let proc_path = format!("/proc/{}", pid);
                if Path::new(&proc_path).exists() {
                    info!("AppLauncher Service: Sending SIGTERM to process {}", pid);
                    let posix_pid = Pid::from_raw(pid as i32);
                    if let Err(e) = kill(posix_pid, Signal::SIGTERM) {
                        error!("AppLauncher Service: Failed to kill process {}: {}", pid, e);
                    }
                }
            }
            pids.clear();
        }
        self.tracked_processes.remove(desktop_file);
        self.broadcast_message(TOPIC_STATUS, DesktopFileStatusMessage::stopped(desktop_file));
    }
}

impl MessageHandler<FfiEnvelopePayload<DesktopFileCommandMessage>> for AppLauncherService {
    fn handle_message(&self, message: FfiEnvelopePayload<DesktopFileCommandMessage>) {
        info!("handle_message: {message:?}");
        match message.action {
            DesktopFileCommandAction::Exec => {
                self.handle_exec(&message.desktop_file);
            }
            DesktopFileCommandAction::ExecStart => {
                self.handle_exec(&message.desktop_file);
            }
            DesktopFileCommandAction::ExecReload => {
                self.handle_exec(&message.desktop_file);
            }
            DesktopFileCommandAction::Terminate => {
                self.handle_terminate(&message.desktop_file);
            }
        }
    }

    fn accept_topic(&self, topic: &str) -> bool {
        topic == TOPIC_COMMAND
    }
}

impl MessageBroadcaster<DesktopFileStatusMessage> for AppLauncherService {}

impl PluginMetaGetter for AppLauncherService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for AppLauncherService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}
