use super::color::convert_color;
use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::SetErrorCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_set_error(message: SetErrorCommandMessage) {
    ensure_hyprland_instance_signature();
    let color = convert_color(message.color);
    if let Err(error) = hyprland::ctl::set_error::call_async(color, message.message).await {
        error!("Hyprland set error failed: {error}");
    }
}
