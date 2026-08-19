use crate::service::TerminalCommandService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_model_mcp::ToolAnnotations;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_terminal_command_model::TerminalCommandLaunchArgs;
use smearor_terminal_command_model::TerminalCommandRestartArgs;
use smearor_terminal_command_model::TerminalCommandTerminateArgs;

impl McpCapabilitiesRegistrator for TerminalCommandService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let running_resource = RegisterResourceMessage::new(
            "terminal_command://running",
            "Running Terminal Commands",
            "List of currently running tracked terminal commands with their PIDs and termination policy.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(running_resource);

        let configured_resource = RegisterResourceMessage::new(
            "terminal_command://configured",
            "Configured Terminal Commands",
            "List of all configured terminal commands from services.toml with their command, args, and options.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(configured_resource);

        let launch_schema = serde_json::to_string(&schema_for!(TerminalCommandLaunchArgs)).unwrap_or_default();
        let launch_tool = RegisterToolMessage::new("terminal_command_launch", "Launch a configured terminal command by command_id.", &launch_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(launch_tool);

        let terminate_schema = serde_json::to_string(&schema_for!(TerminalCommandTerminateArgs)).unwrap_or_default();
        let terminate_tool = RegisterToolMessage::new("terminal_command_terminate", "Terminate a running terminal command by command_id.", &terminate_schema)
            .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(terminate_tool);

        let restart_schema = serde_json::to_string(&schema_for!(TerminalCommandRestartArgs)).unwrap_or_default();
        let restart_tool = RegisterToolMessage::new(
            "terminal_command_restart",
            "Restart a terminal command by command_id (terminate then launch).",
            &restart_schema,
        )
        .with_annotations(&ToolAnnotations::destructive());
        broadcaster.broadcast_message_to_topic(restart_tool);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let prompt = RegisterPromptMessage::with_memory(
            "terminal_command_guide",
            "Lists configured terminal commands and launch instructions.",
            &no_args_schema,
            "terminal command preferences and frequently used commands",
            "terminal,command",
        );
        broadcaster.broadcast_message_to_topic(prompt);

        let lifecycle_prompt = RegisterPromptMessage::with_memory(
            "terminal_lifecycle_guide",
            "Returns instructions for the terminal command lifecycle: check configuration before launching, monitor running processes, and handle restarts.",
            &no_args_schema,
            "terminal command lifecycle preferences and restart behavior",
            "terminal,command",
        );
        broadcaster.broadcast_message_to_topic(lifecycle_prompt);
    }
}

impl TerminalCommandService {
    /// Returns a snapshot of all running tracked commands.
    pub fn running_commands_snapshot(&self) -> Vec<(String, Vec<u32>, bool)> {
        self.process_manager
            .labels()
            .into_iter()
            .map(|label| {
                let pids = self.process_manager.pids_by_label(&label);
                let terminate_on_exit = self
                    .process_manager
                    .get_by_label(&label)
                    .first()
                    .map(|(_, p)| p.terminate_on_exit)
                    .unwrap_or(false);
                (label, pids, terminate_on_exit)
            })
            .collect()
    }

    /// Returns a snapshot of all configured commands.
    pub fn configured_commands_snapshot(&self) -> Vec<(String, String, Vec<String>, bool)> {
        self.config
            .commands
            .iter()
            .map(|(command_id, definition)| (command_id.clone(), definition.command.clone(), definition.args.clone(), definition.restart_on_exit))
            .collect()
    }
}
