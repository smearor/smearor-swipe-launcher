use crate::area::area_manager::AreaManager;
use smearor_model_area::AddAreaMessage;
use smearor_model_area::CloseAreaMessage;
use smearor_model_area::OpenAreaMessage;
use smearor_model_area::RemoveAreaMessage;
use smearor_swipe_launcher_plugin_api::AcceptTopic;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::MessageRouter;
use smearor_swipe_launcher_plugin_api::TypedMessage;

impl MessageRouter for AreaManager {
    fn route(&self, envelope: &FfiEnvelope) {
        if envelope.type_id == FfiEnvelopePayload::<AddAreaMessage>::TYPE_ID
            && AcceptTopic::<FfiEnvelopePayload<AddAreaMessage>>::accept_topic(self, envelope.topic.as_str())
        {
            MessageHandler::<FfiEnvelopePayload<AddAreaMessage>>::handle_envelope_message(self, envelope);
            return;
        }
        if envelope.type_id == FfiEnvelopePayload::<CloseAreaMessage>::TYPE_ID
            && AcceptTopic::<FfiEnvelopePayload<CloseAreaMessage>>::accept_topic(self, envelope.topic.as_str())
        {
            MessageHandler::<FfiEnvelopePayload<CloseAreaMessage>>::handle_envelope_message(self, envelope);
            return;
        }
        if envelope.type_id == FfiEnvelopePayload::<OpenAreaMessage>::TYPE_ID
            && AcceptTopic::<FfiEnvelopePayload<OpenAreaMessage>>::accept_topic(self, envelope.topic.as_str())
        {
            MessageHandler::<FfiEnvelopePayload<OpenAreaMessage>>::handle_envelope_message(self, envelope);
            return;
        }
        if envelope.type_id == FfiEnvelopePayload::<RemoveAreaMessage>::TYPE_ID
            && AcceptTopic::<FfiEnvelopePayload<RemoveAreaMessage>>::accept_topic(self, envelope.topic.as_str())
        {
            MessageHandler::<FfiEnvelopePayload<RemoveAreaMessage>>::handle_envelope_message(self, envelope);
            return;
        }
    }
}

impl<T> AcceptTopic<T> for AreaManager {
    fn accept_topic(&self, topic: &str) -> bool {
        topic.starts_with("area.")
    }
}
