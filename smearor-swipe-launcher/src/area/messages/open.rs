use crate::area::area_manager::AreaManager;
use crate::area::backend::AreaBackend;
use smearor_model_area::OpenAreaMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use tracing::debug;
use tracing::error;
use tracing::trace;

impl<B: AreaBackend> MessageHandler<FfiEnvelopePayload<OpenAreaMessage>> for AreaManager<B> {
    fn handle_message(&self, message: FfiEnvelopePayload<OpenAreaMessage>, sender_id: &str) {
        trace!("Opening area: {} from sender: {}", message.area_id, sender_id);
        let area_id = &message.area_id;
        let Some(area_config) = self.config.get_area_config(area_id) else {
            error!("Area config not found for: {area_id}");
            return;
        };
        if let Err(e) = self.add_transient_area(area_id, area_config.clone(), Some(&sender_id)) {
            debug!("Area open {} skipped: {}", area_id, e);
        } else {
            trace!("Successfully opened area {}", area_id);
        }
    }
}
