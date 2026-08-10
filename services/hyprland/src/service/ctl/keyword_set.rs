use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::KeywordSetCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_keyword_set(message: KeywordSetCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::keyword::Keyword::set_async(message.keyword.as_str(), message.value.as_str()).await {
        error!("Hyprland keyword set failed for '{}': {error}", message.keyword);
    }
}
