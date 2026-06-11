use smearor_wrot_rotation::SmearorRotation;

mod config;
mod context;
mod error;
mod messages;
mod meta;
mod plugin;
mod service;
mod widget;

pub use config::PluginConfig;
pub use context::CoreContextVTable;
pub use context::FfiCoreContext;
pub use error::PluginConstructionError;
pub use messages::FfiEnvelope;
pub use messages::FfiEnvelopePayload;
pub use messages::MessageBroadcaster;
pub use messages::MessageHandler;
pub use meta::PluginMeta;
pub use meta::PluginMetaGetter;
pub use meta::PluginMetaRaw;
pub use plugin::LoadedPlugin;
pub use plugin::PluginConstructor;
pub use plugin::PluginVTable;
pub use service::LoadedService;
pub use service::ServiceConstructor;
pub use service::ServiceVTable;
pub use widget::FfiWidget;

pub type Rotation = SmearorRotation;
