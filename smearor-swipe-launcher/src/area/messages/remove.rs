use crate::area::area_manager::AreaManager;
use smearor_model_area::RemoveAreaMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::error;
use tracing::info;

impl MessageHandler<FfiEnvelopePayload<RemoveAreaMessage>> for AreaManager {
    fn handle_message(&self, message: FfiEnvelopePayload<RemoveAreaMessage>, _sender_id: &str) {
        let area_id = &message.area_id;
        info!("Removing area: {}", area_id);
        match self.remove_area(area_id) {
            Ok(_) => info!("Successfully removed area {}", area_id),
            Err(e) => error!("Failed to remove area {}: {}", area_id, e),
        }
    }
}
