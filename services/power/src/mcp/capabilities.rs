use crate::service::PowerService;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use tracing::debug;

impl McpCapabilitiesRegistrator for PowerService {
    fn register_mcp_capabilities(&self) {
        if !self.config.mcp_enabled {
            debug!("Power Service: MCP tool registration disabled by config");
            return;
        }

        let broadcaster = self.get_broadcaster();

        let capabilities_resource = RegisterResourceMessage::new(
            "power://capabilities",
            "Power Capabilities",
            "System power capabilities as reported by systemd-logind.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(capabilities_resource);

        let inhibitors_resource = RegisterResourceMessage::new(
            "power://inhibitors",
            "Power Inhibitors",
            "List of active inhibitor locks blocking power actions.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(inhibitors_resource);

        let scheduled_resource = RegisterResourceMessage::new(
            "power://scheduled_actions",
            "Scheduled Power Actions",
            "Currently scheduled power action, if any.",
            "application/json",
        );
        broadcaster.broadcast_message_to_topic(scheduled_resource);

        let power_action_tool = RegisterToolMessage::new(
            "system_power_action",
            "Executes the desired power action immediately.",
            r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["shutdown", "reboot", "suspend", "hibernate", "lock", "logout"], "description": "The power action to execute" } }, "required": ["action"] }"#,
        );
        broadcaster.broadcast_message_to_topic(power_action_tool);

        let schedule_tool = RegisterToolMessage::new(
            "system_schedule_power_action",
            "Schedules a shutdown or reboot in the future.",
            r#"{ "type": "object", "properties": { "action": { "type": "string", "enum": ["shutdown", "reboot"], "description": "The power action to schedule" }, "delay_minutes": { "type": "integer", "minimum": 1, "description": "Delay in minutes before the action executes" } }, "required": ["action", "delay_minutes"] }"#,
        );
        broadcaster.broadcast_message_to_topic(schedule_tool);

        let cancel_tool = RegisterToolMessage::new(
            "system_cancel_power_action",
            "Cancels a running shutdown timer or scheduled action.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(cancel_tool);

        let uefi_tool = RegisterToolMessage::new(
            "system_reboot_to_uefi",
            "Sets the firmware reboot flag and reboots directly into BIOS/UEFI.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(uefi_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "power_action_guide",
            "Lists available power actions and safety instructions.",
            r#"{ "type": "object", "properties": {} }"#,
            "power action preferences and shutdown confirmation settings",
            "power",
        );
        broadcaster.broadcast_message_to_topic(prompt);

        let safety_prompt = RegisterPromptMessage::with_memory(
            "power_safety_guide",
            "Returns safety instructions for destructive power actions: always confirm with the user before shutdown, reboot, or UEFI reboot.",
            r#"{ "type": "object", "properties": {} }"#,
            "power safety preferences and destructive action confirmation settings",
            "power",
        );
        broadcaster.broadcast_message_to_topic(safety_prompt);
    }
}
