use crate::service::NetworkService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_network_model::NetworkMcpResources;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<NetworkMcpResources> for NetworkService {
    fn get_response(&self, request: &ResourceRequest<NetworkMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        match request.resource {
            NetworkMcpResources::Status => {
                let status = self.state_snapshot();
                let primary = &status.primary_interface;
                let json = serde_json::json!({
                    "interface_type": format!("{:?}", primary.interface_type),
                    "interface_name": primary.interface_name.to_string(),
                    "state": format!("{:?}", primary.state),
                    "ssid": primary.ssid.as_ref().map(|s| s.to_string()),
                    "signal": primary.signal.as_ref().map(|s| *s),
                    "ipv4_address": primary.ipv4_address.as_ref().map(|s| s.to_string()),
                    "ipv6_address": primary.ipv6_address.as_ref().map(|s| s.to_string()),
                    "internet_accessible": primary.internet_accessible,
                    "wifi_enabled": status.wifi_enabled,
                    "wwan_enabled": status.wwan_enabled,
                    "airplane_mode": status.airplane_mode,
                    "received_bytes_per_second": status.received_bytes_per_second,
                    "transmitted_bytes_per_second": status.transmitted_bytes_per_second,
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            NetworkMcpResources::ScanResults => {
                let scan = self.scan_snapshot();
                let aps: Vec<serde_json::Value> = scan
                    .access_points
                    .iter()
                    .map(|ap| {
                        serde_json::json!({
                            "ssid": ap.ssid.to_string(),
                            "bssid": ap.bssid.to_string(),
                            "signal": ap.signal,
                            "frequency": ap.frequency,
                            "security": format!("{:?}", ap.security),
                            "is_connected": ap.is_connected,
                            "is_known": ap.is_known,
                        })
                    })
                    .collect();
                let json = serde_json::json!({
                    "access_points": aps,
                    "scan_time": scan.scan_time.to_string(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
            NetworkMcpResources::VpnProfiles => {
                let vpn = self.vpn_snapshot();
                let profiles: Vec<serde_json::Value> = vpn
                    .profiles
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "name": p.name.to_string(),
                            "vpn_type": p.vpn_type.to_string(),
                            "is_active": p.is_active,
                            "uuid": p.uuid.to_string(),
                        })
                    })
                    .collect();
                let json = serde_json::json!({
                    "profiles": profiles,
                    "last_updated": vpn.last_updated.to_string(),
                });
                InvokeResourceResponse::success(correlation_id, &json.to_string())
            }
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for NetworkService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
