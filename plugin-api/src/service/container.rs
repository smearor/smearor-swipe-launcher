use crate::service::vtable::ServicePluginVTable;

/// Container returned by service plugin constructors.
///
/// `#[repr(C)]` ensures a stable layout. The Host must verify
/// `vtable_version == SERVICE_VTABLE_VERSION` before using the VTable.
#[repr(C)]
pub struct ServicePluginContainer {
    /// Opaque pointer to the service instance (owned by the plugin).
    pub instance: *mut core::ffi::c_void,
    /// Pointer to the static VTable.
    pub vtable: *const ServicePluginVTable,
    /// VTable version. Must match `SERVICE_VTABLE_VERSION`.
    pub vtable_version: u32,
}
