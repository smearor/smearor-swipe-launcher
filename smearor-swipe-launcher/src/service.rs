use crate::context::SimpleCoreContext;
use crate::error::LauncherError;
use libloading::Library;
use serde_json::Value;
use smearor_model_plugin::PluginEntry;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::ServiceConstructor;
use smearor_swipe_launcher_plugin_api::ServiceVTable;
use stabby::libloading::StabbyLibrary;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

/// Represents a loaded background service with its library handle.
pub struct LoadedService {
    _library: Arc<Library>,
    pub instance: *mut core::ffi::c_void,
    pub vtable: *const ServiceVTable,
    core_context: *mut core::ffi::c_void,
}

impl LoadedService {
    pub fn load(service_entry: &PluginEntry, config: &PluginConfig, sender: UnboundedSender<FfiEnvelope>) -> Result<(String, Self), LauncherError> {
        unsafe {
            let path = PathBuf::from(&service_entry.path);
            let library = Arc::new(Library::new(&path)?);

            debug!("Loading service: {:?}", config);
            let constructor = library
                .get_stabbied::<ServiceConstructor>(b"smearor_service_create")
                .map_err(|e| LauncherError::PluginStabbiedLoadError(e.to_string()))?;

            let mut config_ext = config.config.clone();
            config_ext["id"] = Value::String(service_entry.id.clone());
            let config_json = serde_json::to_string(&config_ext)?;
            let config_bytes = config_json.as_bytes();
            let config_ptr = config_bytes.as_ptr() as *const i8;
            let config_len = config_bytes.len();

            let service_id = service_entry.id.clone();
            // Services are shared across all instances, so instance_id is always empty.
            let core_context = SimpleCoreContext::new(sender, tokio::runtime::Handle::current(), service_id.clone(), "");
            let ffi_context = core_context.into_ffi_context();

            let ffi_context_ptr = Box::into_raw(Box::new(ffi_context)) as *mut core::ffi::c_void;
            let result = constructor(config_ptr, config_len, ffi_context_ptr);

            let container_ptr = if result.is_ok() {
                result.unwrap()
            } else {
                let e = result.unwrap_err();
                return Err(LauncherError::PluginConstructionError(e.error, e.message.to_string()));
            };

            if container_ptr.is_null() {
                return Err(LauncherError::PluginConstructionError(
                    smearor_swipe_launcher_plugin_api::PluginConstructionError::Custom,
                    "Service constructor returned null".to_string(),
                ));
            }

            let api_loaded_service = &*(container_ptr as *mut smearor_swipe_launcher_plugin_api::ServiceContainer);
            let service_id = service_entry.id.clone();

            let service = LoadedService {
                _library: library,
                instance: api_loaded_service.instance,
                vtable: api_loaded_service.vtable,
                core_context: ffi_context_ptr,
            };

            Ok((service_id, service))
        }
    }

    pub unsafe fn start(&self) {
        unsafe {
            if !self.vtable.is_null() && !self.instance.is_null() {
                ((*self.vtable).start)(self.instance);
            }
        }
    }

    pub unsafe fn on_message(&self, message: FfiEnvelope) {
        unsafe {
            if !self.vtable.is_null() && !self.instance.is_null() {
                let message_ptr = Box::into_raw(Box::new(message));
                ((*self.vtable).on_message)(self.instance, message_ptr as *mut core::ffi::c_void);
            }
        }
    }

    pub unsafe fn destroy(&self) {
        unsafe {
            if !self.vtable.is_null() && !self.instance.is_null() {
                ((*self.vtable).destroy)(self.instance);
            }
        }
    }
}

impl Drop for LoadedService {
    fn drop(&mut self) {
        unsafe {
            let ffi_ptr = self.core_context;
            if !ffi_ptr.is_null() {
                let ffi = &*(ffi_ptr as *const FfiCoreContext);
                let simple_ptr = ffi.broker.context as *mut SimpleCoreContext;
                if !simple_ptr.is_null() {
                    let _ = Box::from_raw(simple_ptr);
                }
                let _ = Box::from_raw(ffi_ptr as *mut FfiCoreContext);
            }
        }
    }
}
