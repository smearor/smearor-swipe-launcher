use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageBrokerHandle;
use smearor_swipe_launcher_plugin_api::PluginExecutor;
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;

/// Simple implementation of CoreContext for plugins
#[derive(Debug)]
pub struct SimpleCoreContext {
    sender: UnboundedSender<FfiEnvelope>,
    handle: tokio::runtime::Handle,
}

impl SimpleCoreContext {
    pub fn new(sender: UnboundedSender<FfiEnvelope>, handle: tokio::runtime::Handle) -> Self {
        SimpleCoreContext { sender, handle }
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
        }
    }
}

unsafe extern "C" fn broker_send_wrapper(
    context: *const core::ffi::c_void,
    topic_ptr: *const core::ffi::c_char,
    type_id: u64,
    payload: *mut core::ffi::c_void,
    destroy_payload: Option<extern "C" fn(*mut core::ffi::c_void)>,
) {
    let _ = (topic_ptr, type_id, payload, destroy_payload);
    if context.is_null() {
        return;
    }
    unsafe {
        let ctx = &*(context as *const SimpleCoreContext);
        let envelope = FfiEnvelope {
            sender_id: stabby::string::String::from("host"),
            topic: stabby::string::String::from(""),
            type_id,
            payload,
            destroy_payload,
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
