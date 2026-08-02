/// Current version of the service plugin VTable API.
///
/// This number must be incremented whenever `ServicePluginVTable` changes.
pub const SERVICE_VTABLE_VERSION: u32 = 1;

/// Manual FFI-safe vtable for service plugins.
///
/// `#[repr(C)]` ensures a stable layout. **Never reorder or remove fields.**
/// New methods must be appended at the end, and `SERVICE_VTABLE_VERSION`
/// must be incremented.
#[repr(C)]
pub struct ServicePluginVTable {
    /// Destroy the service instance.
    pub destroy: unsafe extern "C" fn(instance: *mut core::ffi::c_void),

    /// Handle an incoming message.
    pub on_message: unsafe extern "C" fn(instance: *mut core::ffi::c_void, message: *mut core::ffi::c_void),

    /// Start the service.
    pub start: unsafe extern "C" fn(instance: *mut core::ffi::c_void),
}
