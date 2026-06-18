use crate::PluginConstructionErrorWrapper;
use crate::PluginMetaGetter;

/// Current version of the service plugin VTable API.
///
/// This number must be incremented whenever `ServiceVTable` changes.
pub const SERVICE_VTABLE_VERSION: u32 = 1;

/// Trait implemented by all service plugins.
///
/// This is a normal Rust trait (not annotated with `#[stabby::stabby]`).
/// The FFI boundary uses the manual `ServiceVTable` below.
///
/// Default (no-op) implementations are provided for `on_message` and `start`,
/// so services only need to override the methods they actually use.
pub trait Service: PluginMetaGetter {
    /// Handle an incoming message from the message broker.
    fn on_message(&mut self, _message: *mut core::ffi::c_void) {}

    /// Start the service after successful construction.
    ///
    /// The Host calls this after `new` returned Ok, allowing the service to spawn
    /// async tasks via the PluginExecutor.
    fn start(&mut self) {}
}

/// Manual FFI-safe vtable for service plugins.
///
/// `#[repr(C)]` ensures a stable layout. **Never reorder or remove fields.**
/// New methods must be appended at the end, and `SERVICE_VTABLE_VERSION`
/// must be incremented.
#[repr(C)]
pub struct ServiceVTable {
    /// Destroy the service instance.
    pub destroy: unsafe extern "C" fn(instance: *mut core::ffi::c_void),

    /// Handle an incoming message.
    pub on_message: unsafe extern "C" fn(instance: *mut core::ffi::c_void, message: *mut core::ffi::c_void),

    /// Start the service.
    pub start: unsafe extern "C" fn(instance: *mut core::ffi::c_void),
}

/// Container returned by service plugin constructors.
///
/// `#[repr(C)]` ensures a stable layout. The Host must verify
/// `vtable_version == SERVICE_VTABLE_VERSION` before using the VTable.
#[repr(C)]
pub struct ServiceContainer {
    /// Opaque pointer to the service instance (owned by the plugin).
    pub instance: *mut core::ffi::c_void,
    /// Pointer to the static VTable.
    pub vtable: *const ServiceVTable,
    /// VTable version. Must match `SERVICE_VTABLE_VERSION`.
    pub vtable_version: u32,
}

/// Constructor signature for service plugins.
///
/// Plugins export this function via `#[stabby::export]` and the `service_plugin!` macro.
///
/// `core_context` is a `*mut FfiCoreContext` cast to `*mut c_void`.
/// The return value is a `*mut ServiceContainer` cast to `*mut c_void`.
/// Using untyped pointers breaks the transitive `IStable` check that
/// would otherwise require `ServiceVTable: IStable`.
pub type ServiceConstructor = extern "C" fn(
    config_json: *const i8,
    config_len: usize,
    core_context: *mut core::ffi::c_void,
) -> stabby::result::Result<*mut core::ffi::c_void, PluginConstructionErrorWrapper>;
