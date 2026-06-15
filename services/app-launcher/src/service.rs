use crate::config::AppLauncherServiceConfig;
use dashmap::DashMap;
use freedesktop_entry_parser::Entry;
use gtk4::glib::ControlFlow;
use gtk4::glib::timeout_add_seconds_local;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use smearor_app_launcher_model::DesktopFileCommandAction;
use smearor_app_launcher_model::DesktopFileCommandMessage;
use smearor_app_launcher_model::DesktopFileStatusMessage;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use which::which;

pub struct AppLauncherService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AppLauncherServiceConfig,
    pub tracked_processes: Arc<DashMap<String, Vec<u32>>>,
}

impl AppLauncherService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let app_launcher_config: AppLauncherServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let service = AppLauncherService {
            meta: PluginMeta::try_from(&config)?,
            config: app_launcher_config,
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
                broadcaster.broadcast_message_to_topic(DesktopFileStatusMessage::stopped(&desktop_file));
            }

            ControlFlow::Continue
        });
    }

    fn handle_exec(&self, desktop_file: &str, follows_rotation: bool) {
        info!("AppLauncher Service: Launching app: {desktop_file} follows_rotation: {follows_rotation}");
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
            let Some(mut program) = raw_args.first().cloned() else {
                error!("Failed to get program attr");
                return;
            };
            trace!("raw_args: {:?}", raw_args);
            raw_args.remove(0);
            info!("AppLauncher Service: smearor_wrot_path: {:?}", self.config.smearor_wrot_path);
            if follows_rotation
                && let Some(rotation) = &self.config.rotation
                && let Some(smearor_wrot_path) = &self.config.smearor_wrot_path
            {
                info!("AppLauncher Service: Launching app with rotation: {desktop_file}");
                let actual_program = program.clone();
                program = self.resolve_path(smearor_wrot_path).to_string_lossy().to_string();

                raw_args.insert(0, actual_program);
                raw_args.insert(0, rotation.to_string());
                raw_args.insert(0, "--rotation".to_string());
            }
            error!("program: {program}");
            error!("args: {:?}", raw_args);
            // Sanitize placeholders like %u, %F
            let clean_args: Vec<String> = raw_args
                .iter()
                .map(|arg| arg.trim().to_string())
                .filter(|arg| !arg.is_empty() && !arg.starts_with('%'))
                .collect();
            error!("clean_args: {:?}", clean_args);

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
                    self.broadcast_message_to_topic(DesktopFileStatusMessage::running(desktop_file));
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
        self.broadcast_message_to_topic(DesktopFileStatusMessage::stopped(desktop_file));
    }

    pub fn resolve_path(&self, executable_name: &str) -> OsString {
        which(&executable_name)
            .map(|path| {
                debug!("Resolved executable '{executable_name}' to: {path:?}");
                path.as_os_str().to_os_string()
            })
            .unwrap_or_else(|e| {
                debug!("Failed to resolve executable '{executable_name}': {}", e);
                executable_name.to_string().into()
            })
    }

    // fn resolve_path(path: &str) -> Option<PathBuf> {
    //     if path.starts_with('~') {
    //         home_dir().map(|home| home.join(path.trim_start_matches("~/")))
    //     } else {
    //         Some(PathBuf::from(path))
    //     }
    // }
}

impl MessageHandler<FfiEnvelopePayload<DesktopFileCommandMessage>> for AppLauncherService {
    fn handle_message(&self, message: FfiEnvelopePayload<DesktopFileCommandMessage>, _sender_id: &str) {
        info!("handle_message: {message:?}");
        match message.action {
            DesktopFileCommandAction::Exec => {
                self.handle_exec(&message.desktop_file, message.follows_rotation);
            }
            DesktopFileCommandAction::ExecStart => {
                self.handle_exec(&message.desktop_file, message.follows_rotation);
            }
            DesktopFileCommandAction::ExecReload => {
                self.handle_exec(&message.desktop_file, message.follows_rotation);
            }
            DesktopFileCommandAction::Terminate => {
                self.handle_terminate(&message.desktop_file);
            }
        }
    }
}

// impl AcceptTopic<FfiEnvelopePayload<DesktopFileCommandMessage>> for AppLauncherService {
//     fn accept_topic(&self, topic: &str) -> bool {
//         topic == TOPIC_COMMAND
//     }
// }

impl MessageBroadcaster<DesktopFileStatusMessage> for AppLauncherService {}

impl MessageTopicBroadcaster<DesktopFileStatusMessage> for AppLauncherService {}

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
