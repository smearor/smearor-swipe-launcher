use crate::service::command::HyprlandCommand;
use crate::service::shared_state::HyprlandSharedState;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::error;

pub(crate) fn spawn_command_worker(
    mut command_receiver: mpsc::UnboundedReceiver<HyprlandCommand>,
    core_context: Option<FfiCoreContext>,
    service_meta: PluginMeta,
    shared_state: Arc<Mutex<HyprlandSharedState>>,
) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(error) => {
                error!("Hyprland Service: failed to create tokio runtime: {error}");
                return;
            }
        };

        rt.block_on(async move {
            while let Some(command) = command_receiver.recv().await {
                command.handle(&core_context, &service_meta, &shared_state).await;
            }
        });
    });
}
