use crate::service::SysinfoService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_sysinfo_model::SysinfoMcpResources;

impl McpResourceHandler<SysinfoMcpResources> for SysinfoService {
    // , correlation_id: &str, _sender_id: &str, uri: &str, _resource: SysinfoMcpResources
    fn get_response(&self, request: &ResourceRequest<SysinfoMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let state = match self.latest_state.read() {
            Ok(state) => state.clone(),
            Err(_) => {
                return InvokeResourceResponse::error(correlation_id, "Failed to read sysinfo state");
            }
        };
        match request.resource {
            SysinfoMcpResources::Cpu => {
                let json = serde_json::json!({
                    "cpu_usage": state.cpu.cpu_usage,
                    "cpu_temperature": state.cpu.cpu_temperature.as_ref().copied(),
                    "temperature_components": state.cpu.temperature_components.iter().map(|c| serde_json::json!({
                        "label": c.label.to_string(),
                        "id": c.id.to_string(),
                        "temperature": c.temperature.as_ref().copied(),
                        "max_temperature": c.max_temperature.as_ref().copied(),
                        "critical_temperature": c.critical_temperature.as_ref().copied(),
                    })).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            SysinfoMcpResources::TemperatureComponents => {
                let json = serde_json::json!({
                    "components": state.cpu.temperature_components.iter().map(|c| serde_json::json!({
                        "label": c.label.to_string(),
                        "id": c.id.to_string(),
                        "temperature": c.temperature.as_ref().copied(),
                        "max_temperature": c.max_temperature.as_ref().copied(),
                        "critical_temperature": c.critical_temperature.as_ref().copied(),
                    })).collect::<Vec<_>>(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            SysinfoMcpResources::Memory => {
                let json = serde_json::json!({
                    "memory_usage": state.memory.memory_usage,
                    "memory_total": state.memory.memory_total,
                    "memory_used": state.memory.memory_used,
                    "memory_available": state.memory.memory_available,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            SysinfoMcpResources::Battery => {
                let json = serde_json::json!({
                    "level": state.battery.level,
                    "status": format!("{:?}", state.battery.status),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            SysinfoMcpResources::Disks => {
                let json = serde_json::json!({
                    "mounts": state.disks.mounts.iter().map(|disk| serde_json::json!({
                        "mount_point": disk.mount_point.to_string(),
                        "usage": disk.usage,
                        "total": disk.total,
                        "used": disk.used,
                        "available": disk.available,
                    })).collect::<Vec<_>>(),
                    "read_bytes_per_second": state.disks.read_bytes_per_second,
                    "write_bytes_per_second": state.disks.write_bytes_per_second,
                })
                .to_string();
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            SysinfoMcpResources::Network => {
                let json = serde_json::json!({
                    "received_bytes_per_second": state.network.received_bytes_per_second,
                    "transmitted_bytes_per_second": state.network.transmitted_bytes_per_second,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            SysinfoMcpResources::Uptime => {
                let json = serde_json::json!({
                    "uptime_seconds": state.uptime.uptime_seconds,
                    "load_average_1_minute": state.uptime.load_average_1_minute,
                    "load_average_5_minute": state.uptime.load_average_5_minute,
                    "load_average_15_minute": state.uptime.load_average_15_minute,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
        }
    }

    fn send_resource_response(&self, response: InvokeResourceResponse, sender_id: &str) {
        self.send_response(response, sender_id);
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for SysinfoService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
