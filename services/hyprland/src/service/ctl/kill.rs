use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::KillCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_kill(_message: KillCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::ctl::kill::call_async().await {
        error!("Hyprland kill failed: {error}");
    }
}
