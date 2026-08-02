use crate::PluginConstructionErrorWrapper;
use crate::PluginMetaGetter;
use crate::widget::widget::WidgetBuilder;

/// Trait implemented by all widget plugins.
///
/// This is a normal Rust trait (not annotated with `#[stabby::stabby]`),
/// because stabby traits cannot contain trait-object parameters.
/// The FFI boundary uses the manual `PluginVTable` below.
///
/// `Plugin` extends `WidgetBuilder` so that `build_widget` is inherited.
/// Plugins only need to implement `on_message` and `start`.
pub trait WidgetPlugin: PluginMetaGetter + WidgetBuilder {
    /// Handle an incoming message from the message broker.
    fn on_message(&mut self, _message: *mut core::ffi::c_void) {}

    /// Start the plugin after successful construction.
    ///
    /// The Host calls this after construction returned Ok, allowing the plugin
    /// to spawn async tasks via the PluginExecutor.
    fn start(&mut self) {}
}

/// Constructor signature for widget plugins.
///
/// Plugins export this function via `#[stabby::export]` and the `widget_plugin!` macro.
///
/// `core_context` is a `*mut FfiCoreContext` cast to `*mut c_void`.
/// The return value is a `*mut PluginContainer` cast to `*mut c_void`.
/// Using untyped pointers breaks the transitive `IStable` check that
/// would otherwise require `PluginVTable: IStable`.
pub type WidgetPluginConstructor = extern "C" fn(
    config_json: *const i8,
    config_len: usize,
    core_context: *mut core::ffi::c_void,
) -> stabby::result::Result<*mut core::ffi::c_void, PluginConstructionErrorWrapper>;
