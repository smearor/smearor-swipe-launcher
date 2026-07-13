use crate::config::AppLauncherServiceConfig;
use dashmap::DashMap;
use freedesktop_entry_parser::Entry;
use glib::MainContext;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use smearor_app_launcher_model::DesktopFileCommandAction;
use smearor_app_launcher_model::DesktopFileCommandMessage;
use smearor_app_launcher_model::DesktopFileStatusMessage;
use smearor_app_launcher_model::SmearorWindowRotationWrapper;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_model_mcp::TOPIC_MCP_INVOKE_RESOURCE;
use smearor_model_mcp::TOPIC_MCP_INVOKE_TOOL;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::trace;
use which::which;

/// A tracked child process with its termination policy.
#[derive(Clone, Debug)]
pub struct TrackedProcess {
    pub pid: u32,
    pub terminate_on_exit: bool,
}

pub struct AppLauncherService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: AppLauncherServiceConfig,
    pub tracked_processes: Arc<DashMap<String, Vec<TrackedProcess>>>,
}

impl AppLauncherService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        smearor_app_launcher_model::register_json_converters(core_context);

        let app_launcher_config: AppLauncherServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|e| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, e.to_string().into()))?;
        let service = AppLauncherService {
            meta: PluginMeta::try_from(&config)?,
            config: app_launcher_config,
            core_context,
            tracked_processes: Arc::new(DashMap::new()),
        };

        let (status_sender, mut status_receiver) = tokio::sync::mpsc::unbounded_channel::<DesktopFileStatusMessage>();
        let tracked_processes_clone = service.tracked_processes.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(e) => {
                    error!("AppLauncher Service: failed to create tokio runtime: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(2));
                loop {
                    interval.tick().await;
                    let mut completed_apps = Vec::new();
                    tracked_processes_clone.retain(|desktop_file, procs| {
                        procs.retain(|tp| {
                            let proc_path = format!("/proc/{}", tp.pid);
                            Path::new(&proc_path).exists()
                        });
                        if procs.is_empty() {
                            completed_apps.push(desktop_file.clone());
                            false
                        } else {
                            true
                        }
                    });
                    for desktop_file in completed_apps {
                        debug!("AppLauncher Service: App exited naturally: {}", desktop_file);
                        let _ = status_sender.send(DesktopFileStatusMessage::stopped(&desktop_file));
                    }
                }
            });
        });

        let broadcaster = service.get_broadcaster();
        MainContext::default().spawn_local(async move {
            while let Some(status) = status_receiver.recv().await {
                broadcaster.broadcast_message_to_topic(status);
            }
        });

        service.register_mcp_capabilities();
        Ok(service)
    }

    pub(crate) fn handle_exec(&self, desktop_file: &str, wrapper: Option<SmearorWindowRotationWrapper>, forked: bool, terminate_on_exit: bool) {
        trace!("AppLauncher Service: Launching app: {desktop_file} (forked={forked})");
        trace!("Using wrapper smearor-wrot: {:?}", wrapper);
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
            trace!("AppLauncher Service: smearor_wrot_path: {:?}", self.config.smearor_wrot_path);
            if let Some(wrapper) = wrapper {
                if let Some(smearor_wrot_path) = &self.config.smearor_wrot_path {
                    trace!("AppLauncher Service: Launching app {desktop_file} with wrapper");
                    let actual_program = program.clone();
                    program = self.resolve_path(smearor_wrot_path).to_string_lossy().to_string();

                    raw_args.insert(0, actual_program);

                    let launcher_rotation = if wrapper.follows_rotation
                        && let Some(rotation) = self.config.rotation
                    {
                        Some(rotation)
                    } else {
                        None
                    };
                    let mut wrapper_args = wrapper.args(launcher_rotation);
                    wrapper_args.append(&mut raw_args);

                    raw_args = wrapper_args;
                }
            }
            trace!("program: {program}");
            trace!("args: {:?}", raw_args);
            // Sanitize placeholders like %u, %F
            let clean_args: Vec<String> = raw_args
                .iter()
                .map(|arg| arg.trim().to_string())
                .filter(|arg| !arg.is_empty() && !arg.starts_with('%'))
                .collect();
            trace!("clean_args: {:?}", clean_args);
            debug!("full command: {program} {}", clean_args.join(" "));

            let mut cmd = Command::new(program.clone());
            cmd.args(&clean_args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());

            if forked {
                // Start the process in a new session so it survives launcher exit.
                // setsid() detaches the process from the launcher's controlling terminal
                // and process group.
                unsafe {
                    cmd.pre_exec(|| {
                        if let Err(e) = nix::unistd::setsid() {
                            return Err(std::io::Error::from_raw_os_error(e as i32));
                        }
                        Ok(())
                    });
                }
            }

            let child = cmd.spawn();

            match child {
                Ok(mut c) => {
                    let pid = c.id();
                    debug!("AppLauncher Service: Successfully spawned {} with PID {} (forked={forked})", program, pid);

                    if forked {
                        // Forked processes are not tracked — they survive launcher exit
                        // and cannot be terminated via long-press. A reaping thread still
                        // runs to prevent zombies while the launcher is alive.
                        debug!("AppLauncher Service: Process PID {} is forked/detached, not tracking", pid);
                    } else {
                        self.tracked_processes
                            .entry(desktop_file.to_string())
                            .or_default()
                            .push(TrackedProcess { pid, terminate_on_exit });
                        self.broadcast_message_to_topic(DesktopFileStatusMessage::running(desktop_file));
                    }

                    // Spawn a reaping thread to call wait() on the child.
                    // Without this, exited children become zombies because
                    // nobody reaps their exit status.
                    std::thread::spawn(move || {
                        if let Err(e) = c.wait() {
                            debug!("AppLauncher Service: wait error for PID {}: {}", pid, e);
                        }
                    });
                }
                Err(e) => {
                    error!("AppLauncher Service: Failed to spawn Command {}: {}", program, e);
                }
            }
        }
    }

    pub(crate) fn handle_terminate(&self, desktop_file: &str) {
        trace!("AppLauncher Service: Terminating app: {desktop_file}");
        if let Some(mut r) = self.tracked_processes.get_mut(desktop_file) {
            let procs = r.value_mut();
            for tp in procs.iter() {
                let proc_path = format!("/proc/{}", tp.pid);
                if Path::new(&proc_path).exists() {
                    trace!("AppLauncher Service: Sending SIGTERM to process {}", tp.pid);
                    let posix_pid = Pid::from_raw(tp.pid as i32);
                    if let Err(e) = kill(posix_pid, Signal::SIGTERM) {
                        error!("AppLauncher Service: Failed to kill process {}: {}", tp.pid, e);
                    }
                }
            }
            procs.clear();
        }
        self.tracked_processes.remove(desktop_file);
        self.broadcast_message_to_topic(DesktopFileStatusMessage::stopped(desktop_file));
    }

    pub fn resolve_path(&self, executable_name: &str) -> OsString {
        which(&executable_name)
            .map(|path| {
                trace!("Resolved executable '{executable_name}' to: {path:?}");
                path.as_os_str().to_os_string()
            })
            .unwrap_or_else(|e| {
                trace!("Failed to resolve executable '{executable_name}': {}", e);
                executable_name.to_string().into()
            })
    }

    pub(crate) fn running_apps_snapshot(&self) -> Vec<(String, Vec<u32>, bool)> {
        self.tracked_processes
            .iter()
            .map(|entry| {
                let desktop_file = entry.key().clone();
                let pids = entry.value().iter().map(|tp| tp.pid).collect::<Vec<_>>();
                let terminate_on_exit = entry.value().first().map(|tp| tp.terminate_on_exit).unwrap_or(false);
                (desktop_file, pids, terminate_on_exit)
            })
            .collect()
    }

    pub(crate) fn available_apps_snapshot(&self) -> Vec<(String, String)> {
        let mut apps = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let search_dirs: [Option<std::path::PathBuf>; 7] = [
            Some(std::path::PathBuf::from("/usr/share/applications")),
            Some(std::path::PathBuf::from("/usr/local/share/applications")),
            Some(std::path::PathBuf::from("/var/lib/flatpak/exports/share/applications")),
            Some(std::path::PathBuf::from("/var/lib/snapd/desktop/applications")),
            dirs::data_dir().map(|d| d.join("applications")),
            dirs::data_dir().map(|d| d.join("flatpak/exports/share/applications")),
            dirs::data_local_dir().map(|d| d.join("applications")),
        ];

        for dir in search_dirs.into_iter().flatten() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                        let path_str = path.to_string_lossy().to_string();
                        if seen.insert(path_str.clone()) {
                            let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path_str.clone());
                            apps.push((path_str, name));
                        }
                    }
                }
            }
        }

        apps.sort_by(|a, b| a.1.cmp(&b.1));
        apps
    }
}

impl MessageHandler<FfiEnvelopePayload<DesktopFileCommandMessage>> for AppLauncherService {
    fn handle_message(&self, message: FfiEnvelopePayload<DesktopFileCommandMessage>, _sender_id: &str) {
        trace!("handle_message: {message:?}");
        match message.action {
            DesktopFileCommandAction::Exec => {
                self.handle_exec(&message.desktop_file, message.wrapper.clone(), message.forked, message.terminate_on_exit);
            }
            DesktopFileCommandAction::ExecStart => {
                self.handle_exec(&message.desktop_file, message.wrapper.clone(), message.forked, message.terminate_on_exit);
            }
            DesktopFileCommandAction::ExecReload => {
                self.handle_exec(&message.desktop_file, message.wrapper.clone(), message.forked, message.terminate_on_exit);
            }
            DesktopFileCommandAction::Terminate => {
                self.handle_terminate(&message.desktop_file);
            }
        }
    }
}

impl MessageBroadcaster for AppLauncherService {}

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

impl Service for AppLauncherService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if !message.is_null() {
            unsafe {
                let envelope = &*(message as *mut FfiEnvelope);
                let topic = envelope.topic.to_string();
                if envelope.type_id == FfiEnvelopePayload::<DesktopFileCommandMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<DesktopFileCommandMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_TOOL && envelope.type_id == FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeToolMessage>>::handle_envelope_message(self, envelope);
                } else if topic == TOPIC_MCP_INVOKE_RESOURCE && envelope.type_id == FfiEnvelopePayload::<InvokeResourceMessage>::TYPE_ID {
                    MessageHandler::<FfiEnvelopePayload<InvokeResourceMessage>>::handle_envelope_message(self, envelope);
                }
            }
        }
    }
}

impl Drop for AppLauncherService {
    fn drop(&mut self) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            debug!("AppLauncher Service: Dropping service, terminating processes with terminate_on_exit=true");
            for entry in self.tracked_processes.iter() {
                for tp in entry.value().iter() {
                    if tp.terminate_on_exit {
                        let proc_path = format!("/proc/{}", tp.pid);
                        if Path::new(&proc_path).exists() {
                            debug!("AppLauncher Service: Sending SIGTERM to process {} on drop", tp.pid);
                            let posix_pid = Pid::from_raw(tp.pid as i32);
                            if let Err(e) = kill(posix_pid, Signal::SIGTERM) {
                                error!("AppLauncher Service: Failed to kill process {} on drop: {}", tp.pid, e);
                            }
                        }
                    } else {
                        debug!("AppLauncher Service: Process {} has terminate_on_exit=false, leaving running", tp.pid);
                    }
                }
            }
        }));

        if let Err(_) = result {
            error!("AppLauncher Service: panic during drop, processes may not have been terminated");
        }
    }
}
