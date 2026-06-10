use smearor_wrot_rotation::SmearorRotation;

mod config;
mod context;
mod error;
mod messages;
mod meta;
mod plugin;
mod widget;

pub use config::PluginConfig;
pub use context::CoreContextVTable;
pub use context::FfiCoreContext;
pub use error::PluginConstructionError;
// pub use error::PluginError;
// pub use error::PluginResult;
pub use messages::FfiEnvelope;
pub use meta::PluginMeta;
pub use meta::PluginMetaRaw;
pub use plugin::LoadedPlugin;
pub use plugin::PluginConstructor;
pub use plugin::PluginVTable;
pub use widget::FfiWidget;

pub type Rotation = SmearorRotation;
