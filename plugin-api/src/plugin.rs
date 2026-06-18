use crate::FfiWidget;
use crate::PluginConstructionErrorWrapper;
use crate::PluginMetaGetter;
use crate::widget::WidgetBuilder;

/// Current version of the widget plugin VTable API.
///
/// This number must be incremented whenever `PluginVTable` changes
/// (new fields added, existing fields reordered, or signatures changed).
/// Plugins and Host compare this version at load time to ensure compatibility.
pub const PLUGIN_VTABLE_VERSION: u32 = 1;

/// Trait implemented by all widget plugins.
///
/// This is a normal Rust trait (not annotated with `#[stabby::stabby]`),
/// because stabby traits cannot contain trait-object parameters.
/// The FFI boundary uses the manual `PluginVTable` below.
///
/// `Plugin` extends `WidgetBuilder` so that `build_widget` is inherited.
/// Plugins only need to implement `on_message` and `start`.
pub trait Plugin: PluginMetaGetter + WidgetBuilder {
    /// Handle an incoming message from the message broker.
    fn on_message(&mut self, _message: *mut core::ffi::c_void) {}

    /// Start the plugin after successful construction.
    ///
    /// The Host calls this after construction returned Ok, allowing the plugin
    /// to spawn async tasks via the PluginExecutor.
    fn start(&mut self) {}
}

/// Manual FFI-safe vtable for widget plugins.
///
/// `#[repr(C)]` ensures a stable layout. **Never reorder or remove fields.**
/// New methods must be appended at the end, and `PLUGIN_VTABLE_VERSION`
/// must be incremented.
#[repr(C)]
pub struct PluginVTable {
    /// Destroy the plugin instance.
    pub destroy: unsafe extern "C" fn(instance: *mut core::ffi::c_void),

    /// Build and return the GTK widget.
    pub build_widget: unsafe extern "C" fn(instance: *mut core::ffi::c_void) -> FfiWidget,

    /// Handle an incoming message.
    pub on_message: unsafe extern "C" fn(instance: *mut core::ffi::c_void, message: *mut core::ffi::c_void),

    /// Start the plugin.
    pub start: unsafe extern "C" fn(instance: *mut core::ffi::c_void),
}

/// Container returned by widget plugin constructors.
///
/// `#[repr(C)]` ensures a stable layout. The Host must verify
/// `vtable_version == PLUGIN_VTABLE_VERSION` before using the VTable.
#[repr(C)]
pub struct PluginContainer {
    /// Opaque pointer to the plugin instance (owned by the plugin).
    pub instance: *mut core::ffi::c_void,
    /// Pointer to the static VTable (lives for the duration of the shared library).
    pub vtable: *const PluginVTable,
    /// VTable version. Must match `PLUGIN_VTABLE_VERSION`.
    pub vtable_version: u32,
}

/// Constructor signature for widget plugins.
///
/// Plugins export this function via `#[stabby::export]` and the `widget_plugin!` macro.
///
/// `core_context` is a `*mut FfiCoreContext` cast to `*mut c_void`.
/// The return value is a `*mut PluginContainer` cast to `*mut c_void`.
/// Using untyped pointers breaks the transitive `IStable` check that
/// would otherwise require `PluginVTable: IStable`.
pub type PluginConstructor = extern "C" fn(
    config_json: *const i8,
    config_len: usize,
    core_context: *mut core::ffi::c_void,
) -> stabby::result::Result<*mut core::ffi::c_void, PluginConstructionErrorWrapper>;
