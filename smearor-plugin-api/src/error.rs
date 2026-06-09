use abi_stable::StableAbi;
use abi_stable::std_types::RString;
use miette::Diagnostic;
use thiserror::Error;

pub type PluginResult<T> = Result<T, PluginError>;

#[derive(Debug, Error, Diagnostic)]
pub enum PluginError {
    #[error("Invalid configuration JSON: {0}")]
    InvalidConfig(String),

    #[error("Failed to build widget: {0}")]
    WidgetBuildError(String),

    #[error("Action failed: {0}")]
    ActionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[repr(C)]
#[derive(StableAbi, Debug, Error)]
pub enum PluginConstructionError {
    #[error("Failed to parse meta data: {0}")]
    FailedToParseMetaData(RString),

    #[error("Failed to parse widget config: {0}")]
    FailedToParseWidgetConfig(RString),

    #[error("Failed to create runtime: {0}")]
    FailedToCreateRuntime(RString),

    #[error("Config JSON is null")]
    ConfigJsonIsNull,

    #[error("Invalid UTF-8 config: {0}")]
    InvalidUtf8Config(RString),

    #[error("Failed to parse config: {0}")]
    FailedToParseConfig(RString),
}
