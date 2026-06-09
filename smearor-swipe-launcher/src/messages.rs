use crate::application::LauncherApplication;
use smearor_plugin_api::CoreMessage;
use tracing::debug;
use tracing::info;

impl LauncherApplication {
    pub fn handle_messages(&mut self) {
        while let Ok(message) = self.message_receiver.try_recv() {
            debug!("Received message from plugin: {:?}", message);

            match message {
                CoreMessage::RequestClose => {
                    info!("Plugin requested close");
                }
                CoreMessage::TriggerParentMenu => {
                    info!("Plugin requested parent menu");
                }
                CoreMessage::EmitNotification { title, body } => {
                    info!("Plugin emitted notification: {} - {}", title.to_string(), body.to_string());
                }
                CoreMessage::ScrollToPosition { position } => {
                    info!("Plugin requested scroll to position: {}", position);
                }
            }
        }
    }
}
