use crate::config::PluginEntry;
use crate::context::SimpleCoreContext;
use crate::error::LauncherError;
use crate::error::Result;
use abi_stable::RRef;
use abi_stable::std_types::RResult;
use libloading::Library;
use libloading::Symbol;
use serde_json::Value;
use smearor_plugin_api::FfiEnvelope;
use smearor_plugin_api::PluginConfig;
use smearor_plugin_api::ServiceVTable;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::debug;

/// Represents a loaded background service with its library handle
#[derive(Clone)]
pub struct LoadedService {
    _library: Arc<Library>,
    pub instance: *mut (),
    pub vtable: RRef<'static, ServiceVTable>,
    core_context: Arc<*mut ()>,
}

impl LoadedService {
    pub fn load(service_entry: &PluginEntry, config: &PluginConfig, sender: Sender<FfiEnvelope>) -> Result<(String, Self)> {
        unsafe {
            let path = PathBuf::from(&service_entry.path);
            let library = Arc::new(Library::new(&path)?);

            debug!("Loading service: {:?}", config);
            let constructor: Symbol<smearor_plugin_api::ServiceConstructor> = library.get(b"smearor_service_create")?;

            let mut config_ext = config.config.clone();
            config_ext["id"] = Value::String(service_entry.id.clone());
            let config_json = serde_json::to_string(&config.config)?;
            let config_bytes = config_json.as_bytes();
            let config_ptr = config_bytes.as_ptr() as *const i8;
            let config_len = config_bytes.len();

            let core_context = SimpleCoreContext::new(sender);
            let ffi_context = core_context.into_ffi_context();

            let result = constructor(config_ptr, config_len, ffi_context);

            let api_loaded_service = match result {
                RResult::ROk(service) => service,
                RResult::RErr(err) => {
                    return Err(LauncherError::PluginConstructorNull(err.to_string()));
                }
            };

            let service_id = {
                let id_rstring = (api_loaded_service.vtable.get().get_id)(api_loaded_service.service_instance);
                id_rstring.to_string()
            };

            let service = LoadedService {
                _library: library,
                instance: api_loaded_service.service_instance,
                vtable: api_loaded_service.vtable,
                core_context: Arc::new(ffi_context.core_obj),
            };

            Ok((service_id, service))
        }
    }

    pub unsafe fn on_message(&self, message: FfiEnvelope) {
        unsafe {
            (self.vtable.get().on_message)(self.instance, message);
        }
    }

    pub unsafe fn destroy(&self) {
        unsafe {
            (self.vtable.get().destroy)(self.instance);
        }
    }
}

impl Drop for LoadedService {
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
