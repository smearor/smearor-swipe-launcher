use abi_stable::RRef;
use smearor_swipe_launcher_plugin_api::CoreContextVTable;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;

/// Simple implementation of CoreContext for plugins
#[derive(Debug)]
pub struct SimpleCoreContext {
    sender: UnboundedSender<FfiEnvelope>,
}

impl SimpleCoreContext {
    pub fn new(sender: UnboundedSender<FfiEnvelope>) -> Self {
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
        if let Err(e) = context.sender.send(message) {
            error!("Failed to send message to core: {}", e);
        }
    }
}
