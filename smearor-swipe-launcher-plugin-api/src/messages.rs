use crate::FfiCoreContext;
use crate::PluginMeta;
use crate::PluginMetaGetter;
use abi_stable::StableAbi;
use abi_stable::std_types::RString;
use serde::Serialize;
use serde::de;
use serde::de::DeserializeOwned;
use std::fmt::Display;
use std::ops::Deref;
use tracing::debug;
use tracing::error;

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct FfiEnvelope {
    pub sender_id: RString,
    pub topic: RString,
    pub payload: RString, // Serialized JSON payload
}

impl FfiEnvelope {
    pub fn payload<T>(&self) -> Result<T, serde_json::Error>
    where
        T: de::DeserializeOwned,
    {
        let payload = self.payload.to_string();
        serde_json::from_str(&payload)
    }
}

impl Display for FfiEnvelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FfiEnvelope {{ sender_id: {}, topic: {}, payload: {} }}", self.sender_id, self.topic, self.payload)
    }
}

#[derive(Debug, Clone)]
pub struct FfiEnvelopePayload<T>(pub T);

impl<T> From<T> for FfiEnvelopePayload<T> {
    fn from(payload: T) -> Self {
        Self(payload)
    }
}

impl<T> Deref for FfiEnvelopePayload<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> TryFrom<FfiEnvelope> for FfiEnvelopePayload<T>
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn try_from(message: FfiEnvelope) -> Result<Self, Self::Error> {
        let payload = message.payload.to_string();
        Ok(FfiEnvelopePayload(serde_json::from_str(&payload)?))
    }
}

pub trait MessageHandler<T: TryFrom<FfiEnvelope>> {
    fn handle_message(&self, message: T);

    fn handle_envelope_message(&self, envelope: FfiEnvelope)
    where
        <T as TryFrom<FfiEnvelope>>::Error: Display,
    {
        if !self.accept_topic(envelope.topic.as_str()) {
            debug!("Topic {} not accepted by message handler", envelope.topic);
            return;
        }
        match T::try_from(envelope) {
            Ok(message) => {
                debug!("Handle message payload");
                self.handle_message(message);
            }
            Err(e) => {
                error!("Failed to parse message payload: {}", e);
            }
        }
        // if let Ok(message) = T::try_from(envelope) {
        //     debug!("Topic {} not accepted by message handler", envelope.topic);
        //     self.handle_message(message);
        // }
    }

    fn accept_topic(&self, topic: &str) -> bool {
        true
    }
}

pub struct MessageBroadcasterInner {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
}

impl<T> MessageBroadcaster<T> for MessageBroadcasterInner
where
    Self: PluginMetaGetter + AsRef<Option<FfiCoreContext>>,
    T: Serialize,
{
}

impl PluginMetaGetter for MessageBroadcasterInner {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for MessageBroadcasterInner {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

pub trait MessageBroadcaster<T>
where
    Self: PluginMetaGetter + AsRef<Option<FfiCoreContext>>,
    T: Serialize,
{
    fn get_broadcaster(&self) -> MessageBroadcasterInner {
        MessageBroadcasterInner {
            meta: self.meta(),
            core_context: self.as_ref().clone(),
        }
    }

    fn broadcast_message(&self, topic: &str, message: T) {
        let meta = self.meta();
        if let Ok(payload) = serde_json::to_string(&message) {
            self.broadcast_envelope(FfiEnvelope {
                sender_id: meta.id.clone(),
                topic: RString::from(topic),
                payload: RString::from(payload),
            });
        }
    }

    fn broadcast_envelope(&self, envelope: FfiEnvelope) {
        if let Some(ffi_core_context) = self.as_ref() {
            unsafe {
                (ffi_core_context.vtable.get().send_message)(ffi_core_context.core_obj, envelope);
            }
        }
    }
}
