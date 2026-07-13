use crate::config::CommandDefinition;
use crate::service::tracked_process::TrackedProcess;
use dashmap::DashMap;
use smearor_terminal_command_model::TerminalCommandStatusMessage;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use tracing::error;

/// Spawns the background reaper thread that monitors tracked processes.
///
/// Every 2 seconds it checks whether tracked PIDs still exist in `/proc`.
/// Exited commands trigger a `Stopped` status broadcast via `status_sender`
/// and, if `restart_on_exit` is set, a restart message via `restart_sender`.
pub fn spawn_reaper_thread(
    tracked_processes: Arc<DashMap<String, Vec<TrackedProcess>>>,
    commands: Arc<HashMap<String, CommandDefinition>>,
    status_sender: tokio::sync::mpsc::UnboundedSender<TerminalCommandStatusMessage>,
    restart_sender: tokio::sync::mpsc::UnboundedSender<String>,
) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                error!("TerminalCommand Service: failed to create tokio runtime: {e}");
                return;
            }
        };
        rt.block_on(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                let mut completed_commands = Vec::new();
                tracked_processes.retain(|command_id, procs| {
                    procs.retain(|tp| {
                        let proc_path = format!("/proc/{}", tp.pid);
                        Path::new(&proc_path).exists()
                    });
                    if procs.is_empty() {
                        completed_commands.push(command_id.clone());
                        false
                    } else {
                        true
                    }
                });
                for command_id in completed_commands {
                    debug!("TerminalCommand Service: Command exited naturally: {}", command_id);
                    let _ = status_sender.send(TerminalCommandStatusMessage::stopped(&command_id));
                    if let Some(definition) = commands.get(&command_id) {
                        if definition.restart_on_exit {
                            debug!("TerminalCommand Service: Scheduling restart for command: {}", command_id);
                            let _ = restart_sender.send(command_id);
                        }
                    }
                }
            }
        });
    });
}
