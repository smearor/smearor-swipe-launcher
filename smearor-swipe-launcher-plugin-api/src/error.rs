use abi_stable::StableAbi;
use abi_stable::std_types::RString;
use thiserror::Error;

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

    #[error("{0}")]
    Custom(RString),
}
