use crate::service::ensure_hyprland_instance_signature;
use smearor_hyprland_model::PluginUnloadCommandMessage;
use tracing::error;

pub(crate) async fn handle_ctl_plugin_unload(message: PluginUnloadCommandMessage) {
    ensure_hyprland_instance_signature();
    let path = std::path::Path::new(&message.name);
    if let Err(error) = hyprland::ctl::plugin::unload_async(path).await {
        error!("Hyprland plugin unload failed: {error}");
    }
}
