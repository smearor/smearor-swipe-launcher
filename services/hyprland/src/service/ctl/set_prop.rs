use super::prop_type::convert_prop_type;
use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::SetPropCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_set_prop(message: SetPropCommandMessage) {
    ensure_hyprland_instance_signature();
    let prop = convert_prop_type(message.prop);
    if let Err(error) = hyprland::ctl::set_prop::call_async(message.identifier, prop, message.lock).await {
        error!("Hyprland set prop failed: {error}");
    }
}
