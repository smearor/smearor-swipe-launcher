use smearor_mcp_server::LogBuffer;
use smearor_mcp_server::LogEntry;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::LogForwardHandle;
use smearor_swipe_launcher_plugin_api::MessageBrokerHandle;
use smearor_swipe_launcher_plugin_api::PluginExecutor;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::mpsc::UnboundedSender;
use tracing::Level;
use tracing::error;

use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;

pub static GLOBAL_JSON_CONVERTER_REGISTRY: OnceLock<Arc<JsonConverterRegistry>> = OnceLock::new();

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
    log_buffer: Option<Arc<LogBuffer>>,
}

impl SimpleCoreContext {
    pub fn new(
        sender: UnboundedSender<FfiEnvelope>,
        handle: tokio::runtime::Handle,
        plugin_id: String,
        instance_id: &str,
        log_buffer: Option<Arc<LogBuffer>>,
    ) -> Self {
        let sender_id = if instance_id.is_empty() {
            plugin_id
        } else {
            format!("{}:{}", instance_id, plugin_id)
        };
        SimpleCoreContext {
            sender,
            handle,
            sender_id,
            log_buffer,
        }
    }

    pub fn into_ffi_context(self) -> FfiCoreContext {
        let has_log_buffer = self.log_buffer.is_some();
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
            log_forward: if has_log_buffer {
                Some(LogForwardHandle {
                    context: context_ptr,
                    forward: log_forward_wrapper,
                })
            } else {
                None
            },
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
    clone_payload: Option<extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void>,
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
        let envelope = FfiEnvelope::builder()
            .sender_id(ctx.sender_id.clone())
            .target_instance_id(target_instance_id.as_str())
            .topic(topic)
            .type_id(type_id)
            .payload(payload)
            .destroy_payload(destroy_payload)
            .clone_payload(clone_payload)
            .build();
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

/// FFI callback invoked by plugin `LogForwardLayer` for each tracing event.
///
/// Converts the raw FFI parameters into a `LogEntry` and pushes it into the
/// host's `LogBuffer`.
unsafe extern "C" fn log_forward_wrapper(
    context: *const core::ffi::c_void,
    level: u8,
    target_ptr: *const u8,
    target_len: usize,
    message_ptr: *const u8,
    message_len: usize,
    file_ptr: *const u8,
    file_len: usize,
    line: u32,
    timestamp_ms: u64,
) {
    if context.is_null() {
        return;
    }
    let ctx = unsafe { &*(context as *const SimpleCoreContext) };
    let Some(ref buffer) = ctx.log_buffer else { return };

    let target = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(target_ptr, target_len)) };
    let message = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(message_ptr, message_len)) };
    let file = if !file_ptr.is_null() && file_len > 0 {
        Some(unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(file_ptr, file_len)) }.to_string())
    } else {
        None
    };

    let level = match level {
        1 => Level::ERROR,
        2 => Level::WARN,
        3 => Level::INFO,
        4 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let entry = LogEntry {
        timestamp_ms,
        level,
        target: target.to_string(),
        message: message.to_string(),
        fields: Vec::new(),
        file,
        line: if line > 0 { Some(line) } else { None },
    };

    buffer.push(entry);
}
