use crate::PluginConstructionErrorWrapper;
use crate::PluginMetaGetter;

/// Trait implemented by all service plugins.
///
/// This is a normal Rust trait (not annotated with `#[stabby::stabby]`).
/// The FFI boundary uses the manual `ServiceVTable` below.
///
/// Default (no-op) implementations are provided for `on_message` and `start`,
/// so services only need to override the methods they actually use.
pub trait ServicePlugin: PluginMetaGetter {
    /// Handle an incoming message from the message broker.
    fn on_message(&mut self, _message: *mut core::ffi::c_void) {}

    /// Start the service after successful construction.
    ///
    /// The Host calls this after `new` returned Ok, allowing the service to spawn
    /// async tasks via the PluginExecutor.
    fn start(&mut self) {}
}

/// Constructor signature for service plugins.
///
/// Plugins export this function via `#[stabby::export]` and the `service_plugin!` macro.
///
/// `core_context` is a `*mut FfiCoreContext` cast to `*mut c_void`.
/// The return value is a `*mut ServiceContainer` cast to `*mut c_void`.
/// Using untyped pointers breaks the transitive `IStable` check that
/// would otherwise require `ServiceVTable: IStable`.
pub type ServicePluginConstructor = extern "C" fn(
    config_json: *const i8,
    config_len: usize,
    core_context: *mut core::ffi::c_void,
) -> stabby::result::Result<*mut core::ffi::c_void, PluginConstructionErrorWrapper>;
