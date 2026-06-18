use crate::FfiCoreContext;
use crate::MessageBrokerHandle;
use crate::PluginMeta;
use crate::PluginMetaGetter;
use crate::TypedMessage;
use crate::dummy_broker_send;
use crate::generate_type_id;

/// Trait for messages that can be routed through the Host's message broker.
///
/// All message types in the shared `model` crates implement this trait.
/// The broker routes messages as raw pointers, and receivers down-cast
/// using the stable `type_id`.
///
/// This is a normal Rust trait (not annotated with `#[stabby::stabby]`),
/// because stabby traits cannot contain trait-object parameters in their methods.
pub trait SharedMessage: Send + TypedMessage {
    /// The topic this message is published on.
    fn topic(&self) -> &'static str;
}

/// Trait for types that declare a static topic string.
///
/// This is implemented by message structs in the `model` crates.
pub trait MessageTopic {
    fn topic() -> &'static str;
}

impl MessageTopic for () {
    fn topic() -> &'static str {
        ""
    }
}

/// A default `extern "C"` destructor for boxed messages.
///
/// Pass this as `destroy_payload` to `MessageBrokerHandle::send`
/// when the message was allocated with `Box::into_raw`.
pub extern "C" fn default_destroy_payload(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// An ABI-safe envelope for a message crossing the FFI boundary.
///
/// The Host constructs this and passes it to plugin/service VTables.
/// Receivers down-cast `payload` using the stable `type_id`.
#[stabby::stabby(no_opt)]
#[derive(Clone)]
pub struct FfiEnvelope {
    pub sender_id: stabby::string::String,
    pub topic: stabby::string::String,
    pub type_id: u64,
    pub payload: *mut core::ffi::c_void,
    pub destroy_payload: Option<extern "C" fn(*mut core::ffi::c_void)>,
}

/// Router trait for dispatching `FfiEnvelope` to the correct handler.
pub trait MessageRouter {
    fn route(&self, envelope: &FfiEnvelope);
}

/// Wrapper for a typed payload inside an `FfiEnvelope`.
///
/// This is used by the `MessageHandler` API to extract the typed payload
/// from an envelope after the Host has validated the `type_id`.
#[derive(Debug, Clone)]
pub struct FfiEnvelopePayload<T>(pub T);

impl<T> std::ops::Deref for FfiEnvelopePayload<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for FfiEnvelopePayload<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Trait for types that can handle a specific message type.
///
/// Implemented by widgets and services that want to receive messages.
pub trait MessageHandler<T: Clone> {
    fn handle_message(&self, message: T, sender_id: &str);

    /// Convenience method called by the Host router.
    fn handle_envelope_message(&self, envelope: &FfiEnvelope) {
        let sender_id = envelope.sender_id.to_string();
        if !envelope.payload.is_null() {
            unsafe {
                if let Some(payload) = (envelope.payload as *mut T).as_ref() {
                    self.handle_message(payload.clone(), &sender_id);
                }
            }
        }
    }
}

/// Trait for types that declare which topics they accept.
pub trait AcceptTopic<T> {
    fn accept_topic(&self, topic: &str) -> bool;
}

/// Legacy struct used by widgets to broadcast messages.
///
/// Plugins clone this struct into closures that send messages on user interaction.
#[derive(Clone, Debug)]
pub struct MessageBroadcasterInner {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
}

impl MessageBroadcasterInner {
    /// Broadcast a message to the broker.
    ///
    /// `T` is the payload type. For ABI-safe transmission, `T` should be a
    /// `#[stabby::stabby]` struct. The payload is boxed and sent as a raw
    /// pointer with the correct `type_id`.
    pub fn broadcast_message<T: Clone + TypedMessage>(&self, topic: &str, payload: &T) {
        if let Some(ctx) = &self.core_context {
            let payload_ptr = Box::into_raw(Box::new(payload.clone())) as *mut core::ffi::c_void;
            let envelope = FfiEnvelope {
                sender_id: stabby::string::String::from(self.meta.id.clone()),
                topic: stabby::string::String::from(topic),
                type_id: T::TYPE_ID,
                payload: payload_ptr,
                destroy_payload: Some(default_destroy_payload),
            };
            ctx.send_message(envelope);
        }
    }

    /// Broadcast a typed message to a specific topic.
    ///
    /// The topic is derived from the `MessageTopic` trait.
    pub fn broadcast_message_to_topic<T: Clone + MessageTopic + TypedMessage>(&self, message: T) {
        self.broadcast_message(T::topic(), &message);
    }

    /// Broadcast a plain string payload.
    ///
    /// Used by generic widgets (e.g. button, clock) that read topic/payload
    /// from config and do not have a typed message struct.
    pub fn broadcast_string(&self, topic: &str, payload: &str) {
        if let Some(ctx) = &self.core_context {
            let boxed = Box::into_raw(Box::new(payload.to_string())) as *mut core::ffi::c_void;
            let envelope = FfiEnvelope {
                sender_id: stabby::string::String::from(self.meta.id.clone()),
                topic: stabby::string::String::from(topic),
                type_id: generate_type_id("std::string::String"),
                payload: boxed,
                destroy_payload: Some(default_destroy_payload),
            };
            ctx.send_message(envelope);
        }
    }
}

/// Helper trait for broadcasting typed messages to a specific topic.
///
/// Implemented by widgets and services with empty impl blocks.
/// Requires `PluginMetaGetter` and `AsRef<Option<FfiCoreContext>>` so that
/// `get_broadcaster` can construct a `MessageBroadcasterInner` from the
/// plugin's metadata and core context.
pub trait MessageTopicBroadcaster<T: Clone + MessageTopic + TypedMessage>: MessageBroadcaster {
    fn broadcast_message_to_topic(&self, message: T) {
        self.get_broadcaster().broadcast_message_to_topic(message);
    }
}

/// Helper trait for broadcasting typed messages to the Host broker.
///
/// Implemented by widgets and services with empty impl blocks.
/// Requires `PluginMetaGetter` and `AsRef<Option<FfiCoreContext>>` so that
/// `get_broadcaster` can construct a `MessageBroadcasterInner` from the
/// plugin's metadata and core context.
pub trait MessageBroadcaster: PluginMetaGetter + AsRef<Option<FfiCoreContext>> {
    fn broker(&self) -> MessageBrokerHandle {
        MessageBrokerHandle {
            context: core::ptr::null(),
            send: dummy_broker_send,
        }
    }

    fn get_broadcaster(&self) -> MessageBroadcasterInner {
        MessageBroadcasterInner {
            meta: self.meta(),
            core_context: self.as_ref().clone(),
        }
    }

    fn broadcast_message<T: Clone + TypedMessage>(&self, topic: &str, message: T) {
        self.get_broadcaster().broadcast_message(topic, &message);
    }

    fn broadcast_envelope(&self, envelope: FfiEnvelope) {
        if let Some(ctx) = self.as_ref() {
            ctx.send_message(envelope);
        }
    }
}
