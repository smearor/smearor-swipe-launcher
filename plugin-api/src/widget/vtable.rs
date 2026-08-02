use crate::FfiGraphic;
use crate::FfiHtmlString;
use crate::FfiWidget;

/// Current version of the widget plugin VTable API.
///
/// This number must be incremented whenever `PluginVTable` changes
/// (new fields added, existing fields reordered, or signatures changed).
/// Plugins and Host compare this version at load time to ensure compatibility.
pub const PLUGIN_VTABLE_VERSION: u32 = 3;

/// Manual FFI-safe vtable for widget plugins.
///
/// `#[repr(C)]` ensures a stable layout. **Never reorder or remove fields.**
/// New methods must be appended at the end, and `PLUGIN_VTABLE_VERSION`
/// must be incremented.
#[repr(C)]
pub struct WidgetPluginVTable {
    /// Destroy the plugin instance.
    pub destroy: unsafe extern "C" fn(instance: *mut core::ffi::c_void),

    /// Build and return the GTK widget.
    pub build_widget: unsafe extern "C" fn(instance: *mut core::ffi::c_void) -> FfiWidget,

    /// Handle an incoming message.
    pub on_message: unsafe extern "C" fn(instance: *mut core::ffi::c_void, message: *mut core::ffi::c_void),

    /// Start the plugin.
    pub start: unsafe extern "C" fn(instance: *mut core::ffi::c_void),

    /// Render the widget to a graphic (pixel buffer) for headless instances.
    /// Added in v2. Set to `None` for GTK-only widgets.
    pub render_graphic: Option<unsafe extern "C" fn(instance: *mut core::ffi::c_void, width: u32, height: u32) -> FfiGraphic>,

    /// Render the widget to an HTML fragment for web instances.
    /// Added in v3. Set to `None` for GTK-only or Headless-only widgets.
    pub render_html: Option<
        unsafe extern "C" fn(
            instance: *mut core::ffi::c_void,
            instance_id: *const u8,
            instance_id_len: usize,
            plugin_id: *const u8,
            plugin_id_len: usize,
        ) -> FfiHtmlString,
    >,
}
