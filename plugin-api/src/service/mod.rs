//! Service plugin types: trait, VTable, container, and macro.

mod container;
mod r#macro;
mod plugin;
mod vtable;

pub use container::ServicePluginContainer;
pub use plugin::ServicePlugin;
pub use plugin::ServicePluginConstructor;
pub use vtable::SERVICE_VTABLE_VERSION;
pub use vtable::ServicePluginVTable;
