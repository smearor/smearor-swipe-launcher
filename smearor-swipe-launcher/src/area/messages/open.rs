use crate::area::area_manager::AreaManager;
use smearor_model_area::OpenAreaMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::error;
use tracing::info;

impl MessageHandler<FfiEnvelopePayload<OpenAreaMessage>> for AreaManager {
    fn handle_message(&self, message: FfiEnvelopePayload<OpenAreaMessage>, sender_id: &str) {
        info!("Opening area: {} from sender: {}", message.area_id, sender_id);
        let area_id = &message.area_id;
        let Some(area_config) = self.config.get_area_config(area_id) else {
            error!("Area config not found for: {area_id}");
            return;
        };
        if let Err(e) = self.add_transient_area(area_id, area_config.clone(), Some(&sender_id)) {
            error!("Failed to open area {}: {}", area_id, e);
        } else {
            info!("Successfully opened area {}", area_id);
        }
    }
}
