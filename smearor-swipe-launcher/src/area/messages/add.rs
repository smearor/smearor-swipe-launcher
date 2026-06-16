use crate::area::area_manager::AreaManager;
use smearor_model_area::AddAreaMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::error;
use tracing::trace;

impl MessageHandler<FfiEnvelopePayload<AddAreaMessage>> for AreaManager {
    fn handle_message(&self, message: FfiEnvelopePayload<AddAreaMessage>, _sender_id: &str) {
        let area_id = &message.area_id;
        let area_config = message.area_config.clone();
        trace!("Adding area: {}", area_id);
        match self.add_area_from_config(area_id, area_config) {
            Ok(_) => trace!("Successfully removed area {}", area_id),
            Err(e) => error!("Failed to remove area {}: {}", area_id, e),
        }
    }
}
