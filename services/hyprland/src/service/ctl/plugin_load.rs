use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::PluginLoadCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_plugin_load(message: PluginLoadCommandMessage) {
    ensure_hyprland_instance_signature();
    let path = std::path::Path::new(&message.path);
    if let Err(error) = hyprland::ctl::plugin::load_async(path).await {
        error!("Hyprland plugin load failed: {error}");
    }
}
