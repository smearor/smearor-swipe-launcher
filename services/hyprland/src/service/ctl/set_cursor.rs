use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::SetCursorCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_set_cursor(message: SetCursorCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::ctl::set_cursor::call_async(&message.theme, message.size).await {
        error!("Hyprland set cursor failed: {error}");
    }
}
