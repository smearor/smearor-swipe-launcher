use crate::service::TerminalCommandService;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

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

        let launch_tool = RegisterToolMessage::new(
            "terminal_command_launch",
            "Launch a configured terminal command by command_id.",
            r#"{ "type": "object", "properties": { "command_id": { "type": "string", "description": "The configured command identifier" }, "forked": { "type": "boolean", "description": "Whether the process should be detached from the launcher (default: false)" }, "terminate_on_exit": { "type": "boolean", "description": "Whether to terminate the process when the launcher exits (default: true)" } }, "required": ["command_id"] }"#,
        );
        broadcaster.broadcast_message_to_topic(launch_tool);

        let terminate_tool = RegisterToolMessage::new(
            "terminal_command_terminate",
            "Terminate a running terminal command by command_id.",
            r#"{ "type": "object", "properties": { "command_id": { "type": "string", "description": "The configured command identifier" } }, "required": ["command_id"] }"#,
        );
        broadcaster.broadcast_message_to_topic(terminate_tool);

        let restart_tool = RegisterToolMessage::new(
            "terminal_command_restart",
            "Restart a terminal command by command_id (terminate then launch).",
            r#"{ "type": "object", "properties": { "command_id": { "type": "string", "description": "The configured command identifier" }, "forked": { "type": "boolean", "description": "Whether the process should be detached from the launcher (default: false)" }, "terminate_on_exit": { "type": "boolean", "description": "Whether to terminate the process when the launcher exits (default: true)" } }, "required": ["command_id"] }"#,
        );
        broadcaster.broadcast_message_to_topic(restart_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "terminal_command_guide",
            "Lists configured terminal commands and launch instructions.",
            r#"{ "type": "object", "properties": {} }"#,
            "terminal command preferences and frequently used commands",
            "terminal,command",
        );
        broadcaster.broadcast_message_to_topic(prompt);

        let lifecycle_prompt = RegisterPromptMessage::with_memory(
            "terminal_lifecycle_guide",
            "Returns instructions for the terminal command lifecycle: check configuration before launching, monitor running processes, and handle restarts.",
            r#"{ "type": "object", "properties": {} }"#,
            "terminal command lifecycle preferences and restart behavior",
            "terminal,command",
        );
        broadcaster.broadcast_message_to_topic(lifecycle_prompt);
    }
}

impl TerminalCommandService {
    /// Returns a snapshot of all running tracked commands.
    pub fn running_commands_snapshot(&self) -> Vec<(String, Vec<u32>, bool)> {
        self.tracked_processes
            .iter()
            .map(|entry| {
                let command_id = entry.key().clone();
                let pids = entry.value().iter().map(|tp| tp.pid).collect::<Vec<_>>();
                let terminate_on_exit = entry.value().first().map(|tp| tp.terminate_on_exit).unwrap_or(false);
                (command_id, pids, terminate_on_exit)
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
