use dashmap::DashMap;

use crate::FfiEnvelope;

/// Function pointer type for deserializing a JSON string into a typed message pointer.
pub type JsonDeserializerFn = fn(*const u8, usize) -> *mut core::ffi::c_void;

/// Function pointer type for destroying a typed message pointer.
pub type DestroyPayloadFn = extern "C" fn(*mut core::ffi::c_void);

/// FFI callback type for registering a JSON converter in the Host's registry.
///
/// Plugins call this function (provided by the Host via `FfiCoreContext`) to
/// register their message type converters at load time.
pub type RegisterJsonConverterFn =
    unsafe extern "C" fn(topic_ptr: *const u8, topic_len: usize, type_id: u64, deserializer: JsonDeserializerFn, destroy: DestroyPayloadFn);

/// Helper for plugins to register a JSON converter via the Host callback.
///
/// Called by plugins during initialisation (e.g. inside `new()`).
pub fn register_json_converter(
    context: Option<crate::FfiCoreContext>,
    topic: &'static str,
    type_id: u64,
    deserializer: JsonDeserializerFn,
    destroy: DestroyPayloadFn,
) {
    if let Some(ctx) = context {
        if let Some(register) = ctx.register_json_converter {
            unsafe {
                register(topic.as_ptr(), topic.len(), type_id, deserializer, destroy);
            }
        }
    }
}

/// No-op register function used as placeholder when core_context is null.
pub unsafe extern "C" fn dummy_register_json_converter(
    _topic_ptr: *const u8,
    _topic_len: usize,
    _type_id: u64,
    _deserializer: JsonDeserializerFn,
    _destroy: DestroyPayloadFn,
) {
}

/// Entry in the JSON converter registry for a single message type.
pub struct JsonConverterEntry {
    /// Stable type identifier for the message type.
    pub type_id: u64,
    /// Deserializes a JSON byte slice into a boxed message pointer.
    pub deserializer: JsonDeserializerFn,
    /// Destroys the boxed message pointer.
    pub destroy: DestroyPayloadFn,
}

/// Registry that maps message topics to their JSON converter entries.
///
/// Generic widgets (e.g. button) send plain JSON string payloads.
/// The Host uses this registry to convert those strings into typed
/// `FfiEnvelope` messages based on the message topic.
pub struct JsonConverterRegistry {
    converters: DashMap<String, JsonConverterEntry>,
}

impl JsonConverterRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { converters: DashMap::new() }
    }

    /// Register a converter for a message topic.
    pub fn register(&self, topic: &'static str, type_id: u64, deserializer: JsonDeserializerFn, destroy: DestroyPayloadFn) {
        self.converters.insert(
            topic.to_string(),
            JsonConverterEntry {
                type_id,
                deserializer,
                destroy,
            },
        );
    }

    /// Attempt to convert a JSON string payload into a typed `FfiEnvelope`
    /// using the registered converter for the given topic.
    pub fn convert(&self, topic: &str, sender_id: &str, json_str: &str) -> Option<FfiEnvelope> {
        let entry = self.converters.get(topic)?;

        let json_bytes = json_str.as_bytes();
        let ptr = (entry.deserializer)(json_bytes.as_ptr(), json_bytes.len());
        if ptr.is_null() {
            return None;
        }

        Some(FfiEnvelope {
            sender_id: stabby::string::String::from(sender_id),
            topic: stabby::string::String::from(topic),
            type_id: entry.type_id,
            payload: ptr,
            destroy_payload: Some(entry.destroy),
        })
    }
}

impl Default for JsonConverterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can be deserialized from JSON string payloads.
///
/// Implemented via the `impl_json_convertible!` macro. Each message type
/// declares how to construct itself from a `serde_json::Value`.
pub trait JsonConvertible {
    /// Register this message type in the given JSON converter registry.
    fn register_json_converter(registry: &JsonConverterRegistry);
}

/// Implement `JsonConvertible` for a message type via a local wrapper.
///
/// Each invocation creates a unique local struct, avoiding Rust's orphan
/// rule when the trait and the type are defined in different crates.
///
/// # Usage
///
/// ```ignore
/// impl_json_convertible!(OpenAreaMessageConverter, OpenAreaMessage, |json| {
///     OpenAreaMessage::new(json.get("area_id").unwrap().as_str().unwrap())
/// });
/// ```
#[macro_export]
macro_rules! impl_json_convertible {
    ($wrapper:ident, $ty:ty, $from_json:expr) => {
        pub struct $wrapper;

        impl $wrapper {
            /// Register this message type's JSON converter via the Host's FFI callback.
            ///
            /// Called by plugins during initialisation to register converters for
            /// their message types in the Host's `JsonConverterRegistry`.
            pub fn register_in_host(context: Option<$crate::FfiCoreContext>) {
                struct __JsonConverter;
                impl __JsonConverter {
                    fn deserialize(ptr: *const u8, len: usize) -> *mut core::ffi::c_void {
                        if ptr.is_null() {
                            return core::ptr::null_mut();
                        }
                        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
                        let json_str = match std::str::from_utf8(bytes) {
                            Ok(s) => s,
                            Err(_) => return core::ptr::null_mut(),
                        };
                        let json = match serde_json::from_str::<serde_json::Value>(json_str) {
                            Ok(v) => v,
                            Err(_) => return core::ptr::null_mut(),
                        };
                        let msg: $ty = ($from_json)(json);
                        Box::into_raw(Box::new(msg)) as *mut core::ffi::c_void
                    }
                }

                use $crate::MessageTopic;
                use $crate::TypedMessage;
                use $crate::default_destroy_payload;
                use $crate::register_json_converter;

                register_json_converter(
                    context,
                    <$ty as MessageTopic>::topic(),
                    <$ty as TypedMessage>::TYPE_ID,
                    __JsonConverter::deserialize,
                    default_destroy_payload,
                );
            }
        }

        impl $crate::JsonConvertible for $wrapper {
            fn register_json_converter(registry: &$crate::JsonConverterRegistry) {
                struct __JsonConverter;
                impl __JsonConverter {
                    fn deserialize(ptr: *const u8, len: usize) -> *mut core::ffi::c_void {
                        if ptr.is_null() {
                            return core::ptr::null_mut();
                        }
                        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
                        let json_str = match std::str::from_utf8(bytes) {
                            Ok(s) => s,
                            Err(_) => return core::ptr::null_mut(),
                        };
                        let json = match serde_json::from_str::<serde_json::Value>(json_str) {
                            Ok(v) => v,
                            Err(_) => return core::ptr::null_mut(),
                        };
                        let msg: $ty = ($from_json)(json);
                        Box::into_raw(Box::new(msg)) as *mut core::ffi::c_void
                    }
                }

                use $crate::MessageTopic;
                use $crate::TypedMessage;
                use $crate::default_destroy_payload;

                registry.register(
                    <$ty as MessageTopic>::topic(),
                    <$ty as TypedMessage>::TYPE_ID,
                    __JsonConverter::deserialize,
                    default_destroy_payload,
                );
            }
        }
    };
}
