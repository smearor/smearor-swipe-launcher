use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tracing::subscriber::SetGlobalDefaultError;

#[derive(Error, Debug)]
pub enum LauncherError {
    #[error("Failed to load configuration file: {0}")]
    ConfigLoadError(#[from] std::io::Error),

    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(#[from] toml::de::Error),

    #[error("Failed to serialize plugin config: {0}")]
    ConfigSerializeError(#[from] serde_json::Error),

    #[error("Failed to load plugin library: {0}")]
    PluginLoadError(#[from] libloading::Error),

    #[error("Plugin constructor returned null pointer: {0}")]
    PluginConstructorNull(String),

    #[error("Plugin get_id returned null pointer")]
    PluginGetIdNull,

    #[error("Failed to initialize GTK: {0}")]
    GtkInitError(String),

    #[error("Failed to build widget: null pointer returned")]
    WidgetBuildError,

    #[error("Message channel error: {0}")]
    ChannelError(#[from] SendError<FfiEnvelope>),

    #[error("Failed to set global tracing subscriber: {0}")]
    TracingError(#[from] SetGlobalDefaultError),
}

pub type Result<T> = std::result::Result<T, LauncherError>;
