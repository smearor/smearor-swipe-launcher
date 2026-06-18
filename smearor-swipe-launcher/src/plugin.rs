use crate::context::SimpleCoreContext;
use crate::error::LauncherError;
use libloading::Library;
use serde_json::Value;
use smearor_model_plugin::PluginEntry;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiWidget;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructor;
use smearor_swipe_launcher_plugin_api::PluginVTable;
use stabby::libloading::StabbyLibrary;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

/// Represents a loaded plugin with its library handle.
pub struct LoadedPlugin {
    _library: Arc<Library>,
    pub instance: *mut core::ffi::c_void,
    pub vtable: *const PluginVTable,
    core_context: *mut core::ffi::c_void,
}

impl LoadedPlugin {
    pub fn load(plugin_entry: &PluginEntry, config: &PluginConfig, sender: UnboundedSender<FfiEnvelope>) -> Result<(String, Self), LauncherError> {
        unsafe {
            let path = PathBuf::from(&plugin_entry.path);
            let library = Arc::new(Library::new(&path)?);

            debug!("load plugin: {:?}", config);
            let constructor = library
                .get_stabbied::<PluginConstructor>(b"smearor_plugin_create")
                .map_err(|e| LauncherError::PluginStabbiedLoadError(e.to_string()))?;

            let mut config_ext = config.config.clone();
            config_ext["id"] = Value::String(plugin_entry.id.clone());
            let config_json = serde_json::to_string(&config_ext)?;
            let config_bytes = config_json.as_bytes();
            let config_ptr = config_bytes.as_ptr() as *const i8;
            let config_len = config_bytes.len();

            let core_context = SimpleCoreContext::new(sender, tokio::runtime::Handle::current());
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
                    "Plugin constructor returned null".to_string(),
                ));
            }

            let api_loaded_plugin = &*(container_ptr as *mut smearor_swipe_launcher_plugin_api::PluginContainer);
            let plugin_id = plugin_entry.id.clone();

            let plugin = LoadedPlugin {
                _library: library,
                instance: api_loaded_plugin.instance,
                vtable: api_loaded_plugin.vtable,
                core_context: ffi_context_ptr,
            };

            Ok((plugin_id, plugin))
        }
    }

    pub unsafe fn build_widget(&self) -> Option<FfiWidget> {
        unsafe {
            if self.vtable.is_null() || self.instance.is_null() {
                return None;
            }
            let ffi_widget = ((*self.vtable).build_widget)(self.instance);
            if ffi_widget.raw_widget.is_null() { None } else { Some(ffi_widget) }
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

impl Drop for LoadedPlugin {
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
