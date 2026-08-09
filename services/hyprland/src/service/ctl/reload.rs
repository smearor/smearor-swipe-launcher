use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::ReloadCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_reload(_message: ReloadCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::ctl::reload::call_async().await {
        error!("Hyprland reload failed: {error}");
    }
}
