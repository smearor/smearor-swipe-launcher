use crate::service::MprisService;
use smearor_model_mcp::InvokeResourceMessage;
use smearor_model_mcp::InvokeResourceResponse;
use smearor_model_mcp::resources::handler::McpResourceHandler;
use smearor_model_mcp::resources::handler::ResourceRequest;
use smearor_mpris_model::MprisMcpResources;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;

impl McpResourceHandler<MprisMcpResources> for MprisService {
    fn get_response(&self, request: &ResourceRequest<MprisMcpResources>) -> InvokeResourceResponse {
        let correlation_id = request.correlation_id;
        let status = self.status_snapshot();
        match request.resource {
            MprisMcpResources::Status => match &status {
                Some(s) => {
                    let json = serde_json::json!({
                        "has_player": s.has_player,
                        "active_player": s.active_player.as_ref().map(|p| serde_json::json!({
                            "bus_name": p.bus_name.to_string(),
                            "name": p.name.to_string(),
                            "is_active": p.is_active,
                        })).unwrap_or(serde_json::Value::Null),
                        "players": s.players.iter().map(|p| serde_json::json!({
                            "bus_name": p.bus_name.to_string(),
                            "name": p.name.to_string(),
                            "is_active": p.is_active,
                        })).collect::<Vec<_>>(),
                        "playback_status": format!("{:?}", s.playback_status),
                        "metadata": s.metadata.as_ref().map(|m| serde_json::json!({
                            "title": m.title.to_string(),
                            "artist": m.artist.to_string(),
                            "album": m.album.to_string(),
                            "length": m.length,
                            "art_url": m.art_url.as_ref().map(|a| serde_json::Value::String(a.to_string())).unwrap_or(serde_json::Value::Null),
                        })).unwrap_or(serde_json::Value::Null),
                        "position": s.position,
                        "loop_status": format!("{:?}", s.loop_status),
                        "shuffle": s.shuffle,
                        "volume": s.volume,
                    });
                    InvokeResourceResponse::success(correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(correlation_id, "MPRIS status not yet available"),
            },
            MprisMcpResources::Players => match &status {
                Some(s) => {
                    let players: Vec<serde_json::Value> = s
                        .players
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "bus_name": p.bus_name.to_string(),
                                "name": p.name.to_string(),
                                "is_active": p.is_active,
                            })
                        })
                        .collect();
                    let json = serde_json::Value::Array(players);
                    InvokeResourceResponse::success(correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(correlation_id, "MPRIS status not yet available"),
            },
            MprisMcpResources::Playback => match &status {
                Some(s) => {
                    let json = serde_json::json!({
                        "has_player": s.has_player,
                        "playback_status": format!("{:?}", s.playback_status),
                    });
                    InvokeResourceResponse::success(correlation_id, &json.to_string())
                }
                None => InvokeResourceResponse::error(correlation_id, "MPRIS status not yet available"),
            },
            MprisMcpResources::Metadata => match &status {
                Some(s) => match s.metadata.as_ref() {
                    Some(m) => {
                        let json = serde_json::json!({
                            "title": m.title.to_string(),
                            "artist": m.artist.to_string(),
                            "album": m.album.to_string(),
                            "length": m.length,
                            "art_url": m.art_url.as_ref().map(|a| serde_json::Value::String(a.to_string())).unwrap_or(serde_json::Value::Null),
                        });
                        InvokeResourceResponse::success(correlation_id, &json.to_string())
                    }
                    None => InvokeResourceResponse::success(correlation_id, "null"),
                },
                None => InvokeResourceResponse::error(correlation_id, "MPRIS status not yet available"),
            },
        }
    }
}

impl MessageHandler<FfiEnvelopePayload<InvokeResourceMessage>> for MprisService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokeResourceMessage>, sender_id: &str) {
        self.handle_invoke_resource_message(message, sender_id);
    }
}
