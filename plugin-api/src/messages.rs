use crate::FfiCoreContext;
use crate::PluginMeta;
use crate::PluginMetaGetter;
use abi_stable::StableAbi;
use abi_stable::std_types::RString;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use serde_json::json;
use std::fmt::Display;
use std::ops::Deref;
use tracing::error;
use tracing::trace;

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
        T: DeserializeOwned,
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

impl TryFrom<FfiEnvelope> for Value {
    type Error = serde_json::Error;

    fn try_from(envelope: FfiEnvelope) -> Result<Self, Self::Error> {
        let payload: Value = envelope.payload()?;
        Ok(json!({
            "sender_id": Value::String(envelope.sender_id.to_string()),
            "topic": Value::String(envelope.topic.to_string()),
            "payload": payload,
        }))
    }
}
// = note: required for `FfiEnvelope` to implement `Into<serde_json::Value>`
// = note: required for `serde_json::Value` to implement `TryFrom<FfiEnvelope>`

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

pub trait AcceptTopic<T: TryFrom<FfiEnvelope>> {
    fn accept_topic(&self, topic: &str) -> bool;
}

impl<T, M> AcceptTopic<FfiEnvelopePayload<M>> for T
where
    FfiEnvelopePayload<M>: TryFrom<FfiEnvelope>,
    M: MessageTopic,
{
    fn accept_topic(&self, topic: &str) -> bool {
        topic == M::topic()
    }
}

pub trait MessageRouter: AcceptTopic<FfiEnvelope> {
    fn route(&self, envelope: &FfiEnvelope);
}

pub trait MessageHandler<T: TryFrom<FfiEnvelope>>: AcceptTopic<T> {
    fn handle_message(&self, message: T, sender_id: &str);

    fn handle_envelope_message(&self, envelope: FfiEnvelope)
    where
        <T as TryFrom<FfiEnvelope>>::Error: Display,
    {
        if !self.accept_topic(envelope.topic.as_str()) {
            trace!("Topic {} not accepted by message handler", envelope.topic);
            return;
        }
        let sender_id = envelope.sender_id.to_string();
        match T::try_from(envelope) {
            Ok(message) => {
                self.handle_message(message, &sender_id);
            }
            Err(e) => {
                error!("Failed to parse message payload: {}", e);
            }
        }
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

impl<T> MessageTopicBroadcaster<T> for MessageBroadcasterInner
where
    Self: PluginMetaGetter + AsRef<Option<FfiCoreContext>>,
    T: Serialize + MessageTopic,
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

pub trait MessageTopic {
    fn topic() -> &'static str;
}

pub trait MessageTopicBroadcaster<T>: MessageBroadcaster<T>
where
    Self: PluginMetaGetter + AsRef<Option<FfiCoreContext>>,
    T: Serialize + MessageTopic,
{
    fn broadcast_message_to_topic(&self, message: T) {
        let meta = self.meta();
        if let Ok(payload) = serde_json::to_string(&message) {
            self.broadcast_envelope(FfiEnvelope {
                sender_id: meta.id.clone(),
                topic: RString::from(T::topic()),
                payload: RString::from(payload),
            });
        }
    }
}
