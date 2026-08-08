use crate::FfiCoreContext;
use crate::MessageBrokerHandle;
use crate::PluginMeta;
use crate::PluginMetaGetter;
use crate::TypedMessage;
use crate::dummy_broker_send;
use crate::generate_type_id;
use tracing::trace;

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

impl<T: MessageTopic> MessageTopic for FfiEnvelopePayload<T> {
    fn topic() -> &'static str {
        T::topic()
    }
}

/// Box a payload and return a raw pointer suitable for `FfiEnvelope::payload`.
///
/// This is the standard way to transfer ownership of a message into an
/// `FfiEnvelope`. The corresponding `destroy_payload` must be
/// `default_destroy_payload` (or equivalent) to reclaim the allocation.
pub fn box_payload<T>(payload: T) -> *mut core::ffi::c_void {
    Box::into_raw(Box::new(payload)) as *mut core::ffi::c_void
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

/// A generic `extern "C"` clone function for boxed `Clone` messages.
///
/// Pass this as `clone_payload` to `FfiEnvelope` when the message was
/// allocated with `Box::into_raw` and implements `Clone`.
/// This ensures that `FfiEnvelope::clone()` produces a valid payload copy
/// when the host routes the envelope to multiple recipients (e.g. widget instances).
pub extern "C" fn default_clone_payload<T: Clone>(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let value = &*(ptr as *const T);
        Box::into_raw(Box::new(value.clone())) as *mut core::ffi::c_void
    }
}

/// An ABI-safe envelope for a message crossing the FFI boundary.
///
/// The Host constructs this and passes it to plugin/service VTables.
/// Receivers down-cast `payload` using the stable `type_id`.
#[derive(typed_builder::TypedBuilder)]
#[stabby::stabby(no_opt)]
pub struct FfiEnvelope {
    #[builder(setter(into))]
    pub sender_id: stabby::string::String,
    #[builder(setter(into))]
    pub target_instance_id: stabby::string::String,
    #[builder(setter(into))]
    pub topic: stabby::string::String,
    pub type_id: u64,
    pub payload: *mut core::ffi::c_void,
    pub destroy_payload: Option<extern "C" fn(*mut core::ffi::c_void)>,
    pub clone_payload: Option<extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void>,
}

/// SAFETY: `FfiEnvelope` only carries a raw pointer to an ABI-stable shared
/// message allocated on the heap. The actual message types are `Send`, and the
/// lifetime is managed by the `destroy_payload` / `clone_payload` function pointers
/// registered by the sender. Moving the envelope between threads is therefore safe.
unsafe impl Send for FfiEnvelope {}

impl Clone for FfiEnvelope {
    fn clone(&self) -> Self {
        let cloned_payload = if self.payload.is_null() {
            std::ptr::null_mut()
        } else if let Some(clone) = self.clone_payload {
            (clone)(self.payload)
        } else {
            std::ptr::null_mut()
        };
        FfiEnvelope {
            sender_id: self.sender_id.clone(),
            target_instance_id: self.target_instance_id.clone(),
            topic: self.topic.clone(),
            type_id: self.type_id,
            payload: cloned_payload,
            destroy_payload: self.destroy_payload,
            clone_payload: self.clone_payload,
        }
    }
}

/// Router trait for dispatching `FfiEnvelope` to the correct handler.
pub trait MessageRouter {
    fn route(&self, envelope: &FfiEnvelope);
}

/// Wrapper for a typed payload inside an `FfiEnvelope`.
///
/// This is used by the `MessageHandler` API to extract the typed payload
/// from an envelope after the Host has validated the `type_id`.
///
/// `#[repr(transparent)]` guarantees the same memory layout as `T`,
/// allowing safe pointer casts from `*mut T` to `*mut FfiEnvelopePayload<T>`.
#[repr(transparent)]
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

impl<T: TypedMessage> TypedMessage for FfiEnvelopePayload<T> {
    const TYPE_ID: u64 = T::TYPE_ID;
}

/// Trait for types that can handle a specific message type.
///
/// Implemented by widgets and services that want to receive messages.
pub trait MessageHandler<T: Clone> {
    fn handle_message(&self, message: T, sender_id: &str);

    /// Convenience method called by the Host router.
    fn handle_envelope_message(&self, envelope: &FfiEnvelope) {
        let sender_id = envelope.sender_id.to_string();
        trace!(
            "handle_envelope_message: topic={} type_id={} payload_null={}",
            envelope.topic,
            envelope.type_id,
            envelope.payload.is_null()
        );
        if !envelope.payload.is_null() {
            unsafe {
                let payload = (envelope.payload as *mut T).as_ref();
                trace!("handle_envelope_message: payload as_ref is_none={}", payload.is_none());
                if let Some(payload) = payload {
                    trace!("handle_envelope_message: calling handle_message");
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
        self.broadcast_message_to_instance("", topic, payload);
    }

    pub fn broadcast_message_to_instance<T: Clone + TypedMessage>(&self, target_instance_id: &str, topic: &str, payload: &T) {
        if let Some(ctx) = &self.core_context {
            let payload_ptr = box_payload(payload.clone());
            let envelope = FfiEnvelope::builder()
                .sender_id(self.meta.id.clone())
                .target_instance_id(target_instance_id)
                .topic(topic)
                .type_id(T::TYPE_ID)
                .payload(payload_ptr)
                .destroy_payload(Some(default_destroy_payload))
                .clone_payload(Some(default_clone_payload::<T>))
                .build();
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
        self.broadcast_string_to_instance("", topic, payload);
    }

    pub fn broadcast_string_to_instance(&self, target_instance_id: &str, topic: &str, payload: &str) {
        if let Some(ctx) = &self.core_context {
            let boxed = box_payload(payload.to_string());
            let envelope = FfiEnvelope::builder()
                .sender_id(self.meta.id.clone())
                .target_instance_id(target_instance_id)
                .topic(topic)
                .type_id(generate_type_id("std::string::String"))
                .payload(boxed)
                .destroy_payload(Some(default_destroy_payload))
                .clone_payload(Some(default_clone_payload::<String>))
                .build();
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

    fn send_response<T: TypedMessage + SharedMessage + Clone>(&self, message: T, sender_id: &str) {
        let payload_ptr = box_payload(message.clone());
        let sender_id_string = sender_id.to_string();
        let topic = <T as SharedMessage>::topic(&message);
        let envelope = FfiEnvelope::builder()
            .sender_id(self.meta().id)
            .target_instance_id(sender_id_string.as_str())
            .topic(topic)
            .type_id(T::TYPE_ID)
            .payload(payload_ptr)
            .destroy_payload(Some(default_destroy_payload))
            .clone_payload(Some(default_clone_payload::<T>))
            .build();
        if let Some(ctx) = self.as_ref() {
            ctx.send_message(envelope);
        }
    }
}
