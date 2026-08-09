use super::switch_xkb_layout_cmd::convert_switch_xkb_layout_cmd;
use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::SwitchXkbLayoutCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_switch_xkb_layout(message: SwitchXkbLayoutCommandMessage) {
    ensure_hyprland_instance_signature();
    let cmd = convert_switch_xkb_layout_cmd(message.cmd);
    if let Err(error) = hyprland::ctl::switch_xkb_layout::call(&message.device, cmd) {
        error!("Hyprland switch xkb layout failed: {error}");
    }
}
