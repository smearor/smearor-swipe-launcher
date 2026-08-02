use crate::widget::vtable::WidgetPluginVTable;

/// Container returned by widget plugin constructors.
///
/// `#[repr(C)]` ensures a stable layout. The Host must verify
/// `vtable_version == PLUGIN_VTABLE_VERSION` before using the VTable.
#[repr(C)]
pub struct WidgetPluginContainer {
    /// Opaque pointer to the plugin instance (owned by the plugin).
    pub instance: *mut core::ffi::c_void,
    /// Pointer to the static VTable (lives for the duration of the shared library).
    pub vtable: *const WidgetPluginVTable,
    /// VTable version. Must match `PLUGIN_VTABLE_VERSION`.
    pub vtable_version: u32,
}
