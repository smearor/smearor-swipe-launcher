use super::color::convert_color;
use super::notify_icon::convert_notify_icon;
use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::NotifyCommandMessage;
use std::time::Duration;
use tracing::error;

pub(crate) async fn handle_ctl_notify(message: NotifyCommandMessage) {
    ensure_hyprland_instance_signature();
    let icon = convert_notify_icon(message.icon);
    let color = convert_color(message.color);
    let duration = Duration::from_millis(message.time_ms as u64);
    if let Err(error) = hyprland::ctl::notify::call_async(icon, duration, color, message.message).await {
        error!("Hyprland notify failed: {error}");
    }
}
