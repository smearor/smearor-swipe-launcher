use crate::service::SysinfoService;
use smearor_model_mcp::RegisterPromptMessage;
use smearor_model_mcp::RegisterResourceMessage;
use smearor_model_mcp::RegisterToolMessage;
use smearor_swipe_launcher_plugin_api::McpCapabilitiesRegistrator;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;

impl McpCapabilitiesRegistrator for SysinfoService {
    fn register_mcp_capabilities(&self) {
        let broadcaster = self.get_broadcaster();

        let resources = [
            ("sysinfo://cpu", "CPU Status", "Current CPU usage and temperature.", "application/json"),
            (
                "sysinfo://temperature-components",
                "Temperature Components",
                "Lists all available temperature components with label, id, current temperature, max and critical thresholds. Use this to find the correct component name for config filters.",
                "application/json",
            ),
            ("sysinfo://memory", "Memory Status", "Current memory usage and available memory.", "application/json"),
            ("sysinfo://battery", "Battery Status", "Current battery level and charging state.", "application/json"),
            ("sysinfo://disks", "Disk Status", "Per-mount usage and disk throughput.", "application/json"),
            ("sysinfo://network", "Network Status", "Inbound and outbound network throughput.", "application/json"),
            ("sysinfo://uptime", "Uptime Status", "System uptime and load averages.", "application/json"),
        ];
        for (uri, name, description, mime_type) in resources {
            let resource = RegisterResourceMessage::new(uri, name, description, mime_type);
            broadcaster.broadcast_message_to_topic(resource);
        }

        let tool = RegisterToolMessage::new(
            "sysinfo_refresh",
            "Force an immediate refresh of all sysinfo metrics.",
            r#"{ "type": "object", "properties": {} }"#,
        );
        broadcaster.broadcast_message_to_topic(tool);

        let prompt = RegisterPromptMessage::with_memory(
            "system_health_check",
            "Returns a structured system health diagnostic guide: read CPU, memory, temperature, and battery resources and format a concise status report.",
            r#"{ "type": "object", "properties": {} }"#,
            "CPU temperature threshold preference and memory usage warning threshold",
            "cpu,memory,temperature,battery",
        );
        broadcaster.broadcast_message_to_topic(prompt);
    }
}
