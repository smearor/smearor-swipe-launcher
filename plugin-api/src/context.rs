use crate::FfiEnvelope;
use stabby::future::DynFuture;

/// Opaque handle to the Host's message broker, passed to plugins during construction.
///
/// Plugins use this handle to send messages to other plugins via the Host's broker.
///
/// `#[repr(C)]` ensures a stable layout. All fields are raw pointers or function
/// pointers with raw-pointer arguments, avoiding any stabby trait-object limitations.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MessageBrokerHandle {
    /// Opaque pointer to the Host's broker state.
    pub context: *const core::ffi::c_void,
    /// Send a message to the broker.
    ///
    /// `topic_ptr` points to a null-terminated C string.
    /// `type_id` is the stable type identifier (see `type_id::generate_type_id`).
    /// `payload` is an opaque pointer to the message payload.
    /// `destroy_payload` is called by the broker (or the last receiver) to
    /// free the message memory. Pass `null` if the sender retains ownership.
    pub send: unsafe extern "C" fn(
        context: *const core::ffi::c_void,
        topic_ptr: *const core::ffi::c_char,
        type_id: u64,
        payload: *mut core::ffi::c_void,
        destroy_payload: Option<extern "C" fn(*mut core::ffi::c_void)>,
    ),
}

impl std::fmt::Debug for MessageBrokerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageBrokerHandle")
            .field("context", &self.context)
            .field("send", &"<fn>")
            .finish()
    }
}

/// Core context passed to plugins during construction.
///
/// Contains the `MessageBrokerHandle`, `PluginExecutor`, and
/// `register_json_converter` callback needed by plugins to communicate
/// with the Host.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiCoreContext {
    pub broker: MessageBrokerHandle,
    pub executor: PluginExecutor,
    /// Callback to register a JSON converter in the Host's registry.
    ///
    /// Plugins call this during initialisation to register converters for
    /// their message types. May be `None` if the Host does not support
    /// JSON converter registration.
    pub register_json_converter: Option<crate::json_converter::RegisterJsonConverterFn>,
}

/// Delegate for spawning futures on the Host's Tokio runtime.
///
/// The Host passes this struct to every plugin during construction. Plugins call
/// `(executor.spawn)(executor.context, future)` to run async tasks on the Host's
/// worker threads.
///
/// `future` is a `stabby::future::DynFuture<'static, ()>` — an ABI-safe boxed
/// future. Plugins create one via `stabby::boxed::Box::new(async { ... }).into()`.
///
/// `#[repr(C)]` ensures the struct layout is ABI-stable.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PluginExecutor {
    /// Opaque pointer to the Host's `tokio::runtime::Handle`.
    /// The Host guarantees this pointer remains valid for the entire plugin lifetime.
    pub context: *const core::ffi::c_void,
    /// Spawn a future on the Host's async runtime.
    ///
    /// The future is a `stabby::future::DynFuture<'static, ()>` which is
    /// ABI-safe across the FFI boundary.
    pub spawn: unsafe extern "C" fn(context: *const core::ffi::c_void, future: DynFuture<'static, ()>),
}

/// No-op send function used as placeholder when core_context is null.
pub unsafe extern "C" fn dummy_broker_send(
    _context: *const core::ffi::c_void,
    _topic_ptr: *const core::ffi::c_char,
    _type_id: u64,
    _payload: *mut core::ffi::c_void,
    _destroy_payload: Option<extern "C" fn(*mut core::ffi::c_void)>,
) {
}

/// No-op spawn function used as placeholder when core_context is null.
pub unsafe extern "C" fn dummy_executor_spawn(_context: *const core::ffi::c_void, _future: DynFuture<'static, ()>) {}

// SAFETY: The Host guarantees that the context pointer remains valid for the
// entire plugin lifetime and that the broker/executor functions are thread-safe.
unsafe impl Send for FfiCoreContext {}

// SAFETY: The Host guarantees that concurrent access to the broker/executor
// is safe through the underlying synchronization mechanisms.
unsafe impl Sync for FfiCoreContext {}

impl FfiCoreContext {
    /// Send an `FfiEnvelope` through the broker handle.
    ///
    /// This is a convenience wrapper for `(self.broker.send)(...)` that
    /// extracts the fields from the envelope and passes them to the
    /// broker's raw function pointer.
    pub fn send_message(&self, envelope: FfiEnvelope) {
        let topic = std::ffi::CString::new(envelope.topic.to_string()).unwrap_or_default();
        unsafe {
            (self.broker.send)(self.broker.context, topic.as_ptr(), envelope.type_id, envelope.payload, envelope.destroy_payload);
        }
        // Note: envelope.sender_id is dropped here
    }
}

impl std::fmt::Debug for FfiCoreContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfiCoreContext")
            .field("broker", &self.broker)
            .field("executor", &self.executor)
            .field("register_json_converter", &"<fn>")
            .finish()
    }
}

impl std::fmt::Debug for PluginExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginExecutor")
            .field("context", &self.context)
            .field("spawn", &"<fn>")
            .finish()
    }
}
