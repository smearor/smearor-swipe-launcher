use crate::area::area_manager::AreaManager;
use smearor_model_area::CloseAreaMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::error;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<CloseAreaMessage>> for AreaManager {
    fn handle_message(&self, message: FfiEnvelopePayload<CloseAreaMessage>, sender_id: &str) {
        trace!("Closing area: {} from sender: {}", message.area_id, sender_id);
        let area_id = &message.area_id;
        if let Err(e) = self.remove_area(area_id) {
            error!("Failed to open area {}: {}", area_id, e);
        } else {
            trace!("Successfully opened area {}", area_id);
        }
    }
}
