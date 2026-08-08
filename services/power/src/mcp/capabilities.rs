use crate::service::PowerService;
use schemars::schema_for;
use smearor_model_mcp::NoArgs;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_power_model::SystemPowerActionArgs;
use smearor_power_model::SystemSchedulePowerActionArgs;
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

        let power_action_schema = serde_json::to_string(&schema_for!(SystemPowerActionArgs)).unwrap_or_default();
        let power_action_tool = RegisterToolMessage::new("system_power_action", "Executes the desired power action immediately.", &power_action_schema);
        broadcaster.broadcast_message_to_topic(power_action_tool);

        let schedule_schema = serde_json::to_string(&schema_for!(SystemSchedulePowerActionArgs)).unwrap_or_default();
        let schedule_tool = RegisterToolMessage::new("system_schedule_power_action", "Schedules a shutdown or reboot in the future.", &schedule_schema);
        broadcaster.broadcast_message_to_topic(schedule_tool);

        let no_args_schema = serde_json::to_string(&schema_for!(NoArgs)).unwrap_or_default();
        let cancel_tool = RegisterToolMessage::new("system_cancel_power_action", "Cancels a running shutdown timer or scheduled action.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(cancel_tool);

        let uefi_tool =
            RegisterToolMessage::new("system_reboot_to_uefi", "Sets the firmware reboot flag and reboots directly into BIOS/UEFI.", &no_args_schema);
        broadcaster.broadcast_message_to_topic(uefi_tool);

        let prompt = RegisterPromptMessage::with_memory(
            "power_action_guide",
            "Lists available power actions and safety instructions.",
            &no_args_schema,
            "power action preferences and shutdown confirmation settings",
            "power",
        );
        broadcaster.broadcast_message_to_topic(prompt);

        let safety_prompt = RegisterPromptMessage::with_memory(
            "power_safety_guide",
            "Returns safety instructions for destructive power actions: always confirm with the user before shutdown, reboot, or UEFI reboot.",
            &no_args_schema,
            "power safety preferences and destructive action confirmation settings",
            "power",
        );
        broadcaster.broadcast_message_to_topic(safety_prompt);
    }
}
