use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageBrokerHandle;
use smearor_swipe_launcher_plugin_api::PluginExecutor;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;

use crate::json_converter::JsonConverterRegistry;

static GLOBAL_JSON_CONVERTER_REGISTRY: OnceLock<Arc<JsonConverterRegistry>> = OnceLock::new();

/// Initialise the global JSON converter registry used by the FFI callback.
///
/// Must be called before any plugins are loaded.
pub fn initialize_global_json_converter_registry(registry: Arc<JsonConverterRegistry>) -> Result<(), Arc<JsonConverterRegistry>> {
    GLOBAL_JSON_CONVERTER_REGISTRY.set(registry)
}

/// Simple implementation of CoreContext for plugins
#[derive(Debug)]
pub struct SimpleCoreContext {
    sender: UnboundedSender<FfiEnvelope>,
    handle: tokio::runtime::Handle,
    sender_id: String,
}

impl SimpleCoreContext {
    pub fn new(sender: UnboundedSender<FfiEnvelope>, handle: tokio::runtime::Handle, plugin_id: String, instance_id: &str) -> Self {
        let sender_id = if instance_id.is_empty() {
            plugin_id
        } else {
            format!("{}:{}", instance_id, plugin_id)
        };
        SimpleCoreContext { sender, handle, sender_id }
    }

    pub fn into_ffi_context(self) -> FfiCoreContext {
        let context = Box::new(self);
        let context_ptr = Box::into_raw(context) as *mut core::ffi::c_void;

        FfiCoreContext {
            broker: MessageBrokerHandle {
                context: context_ptr,
                send: broker_send_wrapper,
            },
            executor: PluginExecutor {
                context: context_ptr,
                spawn: executor_spawn_wrapper,
            },
            register_json_converter: Some(register_json_converter_wrapper),
        }
    }
}

unsafe extern "C" fn broker_send_wrapper(
    context: *const core::ffi::c_void,
    topic_ptr: *const core::ffi::c_char,
    target_instance_id_ptr: *const core::ffi::c_char,
    type_id: u64,
    payload: *mut core::ffi::c_void,
    destroy_payload: Option<extern "C" fn(*mut core::ffi::c_void)>,
) {
    if context.is_null() {
        return;
    }
    let topic = if topic_ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(topic_ptr).to_string_lossy().into_owned() }
    };
    let target_instance_id = if target_instance_id_ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(target_instance_id_ptr).to_string_lossy().into_owned() }
    };
    unsafe {
        let ctx = &*(context as *const SimpleCoreContext);
        let envelope = FfiEnvelope {
            sender_id: stabby::string::String::from(ctx.sender_id.clone()),
            target_instance_id: stabby::string::String::from(target_instance_id),
            topic: stabby::string::String::from(topic),
            type_id,
            payload,
            destroy_payload,
            clone_payload: None,
        };
        if let Err(e) = ctx.sender.send(envelope) {
            error!("Failed to send message to core: {}", e);
        }
    }
}

unsafe extern "C" fn executor_spawn_wrapper(context: *const core::ffi::c_void, future: stabby::future::DynFuture<'static, ()>) {
    if context.is_null() {
        return;
    }
    let ctx = unsafe { &*(context as *const SimpleCoreContext) };
    ctx.handle.spawn(future);
}

unsafe extern "C" fn register_json_converter_wrapper(
    topic_ptr: *const u8,
    topic_len: usize,
    type_id: u64,
    deserializer: smearor_swipe_launcher_plugin_api::JsonDeserializerFn,
    destroy: smearor_swipe_launcher_plugin_api::DestroyPayloadFn,
) {
    if topic_ptr.is_null() {
        return;
    }
    let topic = unsafe { std::str::from_utf8(std::slice::from_raw_parts(topic_ptr, topic_len)).unwrap_or("invalid-topic") };
    if let Some(registry) = GLOBAL_JSON_CONVERTER_REGISTRY.get() {
        registry.register(topic, type_id, deserializer, destroy);
    }
}
