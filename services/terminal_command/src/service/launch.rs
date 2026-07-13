use crate::service::TerminalCommandService;
use crate::service::tracked_process::TrackedProcess;
use smearor_swipe_launcher_plugin_api::MessageTopicBroadcaster;
use smearor_terminal_command_model::TerminalCommandStatusMessage;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use tracing::debug;
use tracing::error;
use tracing::trace;

impl TerminalCommandService {
    /// Launches a configured command by `command_id`.
    ///
    /// If `forked` is `true`, the process is detached via `setsid()` and not
    /// tracked — it survives launcher exit and cannot be terminated via
    /// long-press. Otherwise the process is tracked with `terminate_on_exit`
    /// controlling whether it is killed when the launcher exits.
    pub(crate) fn handle_launch(&self, command_id: &str, forked: bool, terminate_on_exit: bool) {
        trace!("TerminalCommand Service: Launching command: {command_id} (forked={forked})");

        let Some(definition) = self.config.commands.get(command_id) else {
            error!("TerminalCommand Service: Unknown command_id: {command_id}");
            self.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(command_id));
            return;
        };

        let resolved_program = self.resolve_command(&definition.command);
        if resolved_program.is_empty() {
            error!("TerminalCommand Service: Failed to resolve command: {}", definition.command);
            self.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(command_id));
            return;
        }

        let resolved_path = Path::new(&resolved_program);
        if !resolved_path.exists() {
            error!("TerminalCommand Service: Resolved path does not exist: {}", resolved_program);
            self.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(command_id));
            return;
        }

        if let Some(working_dir) = &definition.working_dir {
            if !working_dir.is_dir() {
                error!("TerminalCommand Service: working_dir is not a directory: {:?}", working_dir);
                self.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(command_id));
                return;
            }
        }

        let mut cmd = Command::new(&resolved_program);
        cmd.args(&definition.args)
            .envs(&definition.env)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(working_dir) = &definition.working_dir {
            cmd.current_dir(working_dir);
        }

        if forked {
            // Start the process in a new session so it survives launcher exit.
            unsafe {
                cmd.pre_exec(|| {
                    if let Err(e) = nix::unistd::setsid() {
                        return Err(std::io::Error::from_raw_os_error(e as i32));
                    }
                    Ok(())
                });
            }
        }

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                debug!("TerminalCommand Service: Successfully spawned {} with PID {} (forked={forked})", resolved_program, pid);

                if forked {
                    debug!("TerminalCommand Service: Process PID {} is forked/detached, not tracking", pid);
                } else {
                    self.tracked_processes
                        .entry(command_id.to_string())
                        .or_default()
                        .push(TrackedProcess { pid, terminate_on_exit });
                    self.broadcast_message_to_topic(TerminalCommandStatusMessage::running(command_id, pid));
                }

                // Spawn a reaping thread to call wait() on the child.
                std::thread::spawn(move || {
                    let mut child = child;
                    if let Err(e) = child.wait() {
                        debug!("TerminalCommand Service: wait error for PID {}: {}", pid, e);
                    }
                });
            }
            Err(e) => {
                error!("TerminalCommand Service: Failed to spawn command {}: {}", resolved_program, e);
                self.broadcast_message_to_topic(TerminalCommandStatusMessage::failed(command_id));
            }
        }
    }
}
