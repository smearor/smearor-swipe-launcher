use super::output_backend::convert_output_backend;
use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::OutputCreateCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_output_create(message: OutputCreateCommandMessage) {
    ensure_hyprland_instance_signature();
    let backend = convert_output_backend(message.backend);
    if let Err(error) = hyprland::ctl::output::create_async(backend, None).await {
        error!("Hyprland output create failed: {error}");
    }
}
