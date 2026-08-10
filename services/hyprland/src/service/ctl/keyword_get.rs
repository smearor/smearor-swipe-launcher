use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::KeywordGetCommandMessage;
use smearor_hyprland_model::KeywordGetResponse;
use smearor_model_mcp::InvokeToolResponse;
use smearor_swipe_launcher_plugin_api::MessageBroadcasterInner;
use tracing::error;

pub(crate) async fn handle_ctl_keyword_get(message: KeywordGetCommandMessage, broadcaster: &MessageBroadcasterInner) {
    ensure_hyprland_instance_signature();
    match hyprland::keyword::Keyword::get_async(message.keyword.as_str()).await {
        Ok(keyword) => {
            let response = KeywordGetResponse {
                keyword: keyword.option.clone(),
                value: keyword.value.to_string(),
                set: keyword.set,
            };
            let json = serde_json::to_string(&response).unwrap_or_default();
            broadcaster.broadcast_message_to_topic(InvokeToolResponse::success(&message.correlation_id, &json));
        }
        Err(error) => {
            error!("Hyprland keyword get failed for '{}': {error}", message.keyword);
            broadcaster.broadcast_message_to_topic(InvokeToolResponse::error(
                &message.correlation_id,
                &format!("Failed to get keyword '{}': {error}", message.keyword),
            ));
        }
    }
}
