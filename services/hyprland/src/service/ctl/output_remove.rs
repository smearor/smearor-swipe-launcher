use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::OutputRemoveCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_output_remove(message: OutputRemoveCommandMessage) {
    ensure_hyprland_instance_signature();
    if let Err(error) = hyprland::ctl::output::remove_async(&message.name).await {
        error!("Hyprland output remove failed: {error}");
    }
}
