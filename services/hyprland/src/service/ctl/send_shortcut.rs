use crate::service::ensure_hyprland_instance_signature;
use hyprland::dispatch::Dispatch;
use hyprland::dispatch::DispatchType;
use smearor_hyprland_model::SendShortcutCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_send_shortcut(message: SendShortcutCommandMessage) {
    ensure_hyprland_instance_signature();
    let shortcut_args = match &message.window {
        Some(window) => format!("{},{},{}", message.mods, message.key, window),
        None => format!("{},{}", message.mods, message.key),
    };
    if let Err(error) = Dispatch::call_async(DispatchType::Custom("sendshortcut", &shortcut_args)).await {
        error!("Hyprland sendshortcut failed: {error}");
    }
}
