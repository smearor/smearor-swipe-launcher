use crate::context::SimpleCoreContext;
use crate::error::LauncherError;
use crate::library_path::resolve_library_path;
use libloading::Library;
use serde_json::Value;
use smearor_model_plugin::PluginEntry;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiGraphic;
use smearor_swipe_launcher_plugin_api::FfiWidget;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::WidgetPluginConstructor;
use smearor_swipe_launcher_plugin_api::WidgetPluginVTable;
use stabby::libloading::StabbyLibrary;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::trace;

/// Represents a loaded plugin with its library handle.
pub struct LoadedPlugin {
    _library: Arc<Library>,
    pub instance: *mut core::ffi::c_void,
    pub vtable: *const WidgetPluginVTable,
    core_context: *mut core::ffi::c_void,
}

impl LoadedPlugin {
    pub fn load(
        plugin_entry: &PluginEntry,
        config: &PluginConfig,
        sender: UnboundedSender<FfiEnvelope>,
        instance_id: &str,
    ) -> Result<(String, Self), LauncherError> {
        unsafe {
            let path = resolve_library_path(&plugin_entry.path, &plugin_entry.name)?;
            let library = Arc::new(Library::new(&path)?);

            trace!("load plugin: {:?}", config);
            let constructor = (&*library)
                .get_stabbied::<WidgetPluginConstructor>(b"smearor_plugin_create")
                .map_err(|e| LauncherError::PluginStabbiedLoadError(e.to_string()))?;

            let mut config_ext = config.config.clone();
            config_ext["id"] = Value::String(plugin_entry.id.clone());
            if let Some(widget) = &plugin_entry.widget {
                config_ext["widget"] = Value::String(widget.clone());
            }
            if let Some(ref span_group) = plugin_entry.span_group {
                config_ext["span_group"] = Value::String(span_group.clone());
            }
            if let Some(span_index) = plugin_entry.span_index {
                config_ext["span_index"] = Value::from(span_index);
            }
            if let Some(span_rows) = plugin_entry.span_rows {
                config_ext["span_rows"] = Value::from(span_rows);
            }
            if let Some(span_cols) = plugin_entry.span_cols {
                config_ext["span_cols"] = Value::from(span_cols);
            }
            let config_json = serde_json::to_string(&config_ext)?;
            let config_bytes = config_json.as_bytes();
            let config_ptr = config_bytes.as_ptr() as *const i8;
            let config_len = config_bytes.len();

            let plugin_id = plugin_entry.id.clone();
            let core_context = SimpleCoreContext::new(sender, tokio::runtime::Handle::current(), plugin_id.clone(), instance_id);
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

            let api_loaded_plugin = &*(container_ptr as *mut smearor_swipe_launcher_plugin_api::WidgetPluginContainer);

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

    /// Render the plugin's graphic at the given dimensions.
    ///
    /// Returns `None` if the plugin does not implement `GraphicRenderer`
    /// (i.e. `render_graphic` is `None` in the VTable).
    pub unsafe fn render_graphic(&self, width: u32, height: u32) -> Option<FfiGraphic> {
        unsafe {
            if self.vtable.is_null() || self.instance.is_null() {
                return None;
            }
            let render_fn = (*self.vtable).render_graphic?;
            let graphic = render_fn(self.instance, width, height);
            if graphic.pixels.is_null() { None } else { Some(graphic) }
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
