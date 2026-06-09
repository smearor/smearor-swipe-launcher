use crate::error::LauncherError;
use crate::error::Result;
use abi_stable::RRef;
use abi_stable::std_types::RResult;
use libloading::Library;
use libloading::Symbol;
use smearor_plugin_api::CoreContextVTable;
use smearor_plugin_api::CoreMessage;
use smearor_plugin_api::FfiCoreContext;
use smearor_plugin_api::FfiWidget;
use smearor_plugin_api::PluginConfig;
use smearor_plugin_api::PluginVTable;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use tracing::error;

/// Simple implementation of CoreContext for plugins
#[derive(Debug)]
pub struct SimpleCoreContext {
    sender: mpsc::Sender<CoreMessage>,
}

impl SimpleCoreContext {
    pub fn new(sender: mpsc::Sender<CoreMessage>) -> Self {
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

unsafe extern "C" fn send_message_wrapper(context: *mut (), message: CoreMessage) {
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

/// Represents a loaded plugin with its library handle
#[derive(Clone)]
pub struct LoadedPlugin {
    _library: Arc<Library>,
    pub instance: *mut (),
    pub vtable: RRef<'static, PluginVTable>,
    core_context: Arc<*mut ()>,
}

impl LoadedPlugin {
    pub fn load(plugin_path: &Path, config: &PluginConfig, sender: mpsc::Sender<CoreMessage>) -> Result<(String, Self)> {
        unsafe {
            let library = Arc::new(Library::new(plugin_path)?);

            let constructor: Symbol<smearor_plugin_api::PluginConstructor> = library.get(b"smearor_plugin_create")?;

            let config_json = serde_json::to_string(&config.config)?;
            let config_bytes = config_json.as_bytes();
            let config_ptr = config_bytes.as_ptr() as *const i8;
            let config_len = config_bytes.len();

            let core_context = SimpleCoreContext::new(sender);
            let ffi_context = core_context.into_ffi_context();

            let result = constructor(config_ptr, config_len, ffi_context);

            let api_loaded_plugin = match result {
                RResult::ROk(plugin) => plugin,
                RResult::RErr(err) => {
                    return Err(LauncherError::PluginConstructorNull);
                }
            };

            let plugin_id = {
                let id_rstring = (api_loaded_plugin.vtable.get().get_id)(api_loaded_plugin.plugin_instance);
                // let id_rstring = ((*api_loaded_plugin.vtable).get_id)(api_loaded_plugin.plugin_instance);
                id_rstring.to_string()
            };

            let plugin = LoadedPlugin {
                _library: library,
                instance: api_loaded_plugin.plugin_instance,
                vtable: api_loaded_plugin.vtable,
                core_context: Arc::new(ffi_context.core_obj),
            };

            Ok((plugin_id, plugin))
        }
    }

    pub unsafe fn build_widget(&self) -> Option<FfiWidget> {
        unsafe {
            let ffi_widget = (self.vtable.get().build_widget)(self.instance);
            if ffi_widget.raw_widget.is_null() { None } else { Some(ffi_widget) }
        }
    }

    pub unsafe fn on_primary_action(&self, rotation: u32) -> i32 {
        unsafe { (self.vtable.get().on_primary_action)(self.instance, rotation) }
    }

    pub unsafe fn on_secondary_action(&self, rotation: u32) -> i32 {
        unsafe { (self.vtable.get().on_secondary_action)(self.instance, rotation) }
    }

    pub unsafe fn destroy(&self) {
        unsafe {
            (self.vtable.get().destroy)(self.instance);
        }
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        if let Ok(context_ptr) = Arc::try_unwrap(self.core_context.clone()) {
            unsafe {
                if !context_ptr.is_null() {
                    let _ = Box::from_raw(context_ptr as *mut SimpleCoreContext);
                }
            }
        }
    }
}
