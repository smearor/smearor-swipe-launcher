use abi_stable::RRef;
use smearor_plugin_api::CoreContextVTable;
use smearor_plugin_api::FfiCoreContext;
use smearor_plugin_api::FfiEnvelope;
use tokio::sync::mpsc::Sender;
use tracing::error;

/// Simple implementation of CoreContext for plugins
#[derive(Debug)]
pub struct SimpleCoreContext {
    sender: Sender<FfiEnvelope>,
}

impl SimpleCoreContext {
    pub fn new(sender: Sender<FfiEnvelope>) -> Self {
        SimpleCoreContext { sender }
    }

    pub fn into_ffi_context(self) -> FfiCoreContext {
        static VTABLE: CoreContextVTable = CoreContextVTable {
            send_message: send_message_wrapper,
        };

        let context = Box::new(self);
        let context_ptr = Box::into_raw(context) as *mut ();

        FfiCoreContext {
            core_obj: context_ptr,
            vtable: RRef::new(&VTABLE),
        }
    }
}

unsafe extern "C" fn send_message_wrapper(context: *mut (), message: FfiEnvelope) {
    if context.is_null() {
        return;
    }

    unsafe {
        let context = &*(context as *const SimpleCoreContext);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let sender = context.sender.clone();
            handle.spawn(async move {
                if let Err(e) = sender.send(message).await {
                    error!("Failed to send message to core asynchronously: {}", e);
                }
            });
        } else {
            if let Err(e) = context.sender.try_send(message) {
                error!("Failed to send message to core via try_send: {}", e);
            }
        }
    }
}
