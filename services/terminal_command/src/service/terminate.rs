use crate::service::TerminalCommandService;
use nix::sys::signal::Signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_terminal_command_model::TerminalCommandStatusMessage;
use std::path::Path;
use std::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::trace;

impl TerminalCommandService {
    /// Terminates all tracked processes for the given `command_id`.
    ///
    /// Sends the configured `kill_signal` (default `SIGTERM`). If the signal
    /// is `SIGTERM`, waits `terminate_timeout_ms` and escalates to `SIGKILL`
    /// for any process still alive.
    pub(crate) fn handle_terminate(&self, command_id: &str) {
        trace!("TerminalCommand Service: Terminating command: {command_id}");

        let Some(definition) = self.config.commands.get(command_id) else {
            debug!("TerminalCommand Service: Unknown command_id for terminate: {command_id}");
            self.broadcast_message_to_topic(TerminalCommandStatusMessage::stopped(command_id));
            return;
        };

        let pids: Vec<u32> = if let Some(mut procs) = self.tracked_processes.get_mut(command_id) {
            let pids = procs.iter().map(|tp| tp.pid).collect();
            procs.clear();
            pids
        } else {
            Vec::new()
        };
        self.tracked_processes.remove(command_id);

        let kill_signal = definition.kill_signal.to_nix_signal();
        let timeout_ms = definition.terminate_timeout_ms;

        for pid in &pids {
            let proc_path = format!("/proc/{}", pid);
            if Path::new(&proc_path).exists() {
                trace!("TerminalCommand Service: Sending {:?} to process {}", kill_signal, pid);
                let posix_pid = Pid::from_raw(*pid as i32);
                if let Err(e) = kill(posix_pid, kill_signal) {
                    error!("TerminalCommand Service: Failed to kill process {}: {}", pid, e);
                }
            }
        }

        // If using SIGTERM, wait for the grace period and escalate to SIGKILL if still alive.
        if kill_signal != Signal::SIGKILL && !pids.is_empty() {
            let pids_clone = pids.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(timeout_ms));
                for pid in &pids_clone {
                    let proc_path = format!("/proc/{}", pid);
                    if Path::new(&proc_path).exists() {
                        debug!("TerminalCommand Service: Process {} still alive after grace period, sending SIGKILL", pid);
                        let posix_pid = Pid::from_raw(*pid as i32);
                        if let Err(e) = kill(posix_pid, Signal::SIGKILL) {
                            error!("TerminalCommand Service: Failed to SIGKILL process {}: {}", pid, e);
                        }
                    }
                }
            });
        }

        self.broadcast_message_to_topic(TerminalCommandStatusMessage::stopped(command_id));
    }
}
