use abi_stable::StableAbi;
use abi_stable::std_types::RString;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[repr(C)]
#[derive(StableAbi, Debug)]
pub enum PluginConstructionError {
    FailedToParseMetaData,
    FailedToParseWidgetConfig,
    FailedToCreateRuntime,
    ConfigJsonIsNull,
    InvalidUtf8Config,
    FailedToParseConfig,
    Custom,
}

#[repr(C)]
#[derive(StableAbi, Debug)]
pub struct PluginConstructionErrorWrapper {
    pub error: PluginConstructionError,
    pub message: RString,
}

impl PluginConstructionErrorWrapper {
    pub fn new(error: PluginConstructionError, message: RString) -> Self {
        Self { error, message }
    }
}

impl Display for PluginConstructionErrorWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}: {}", self.error, self.message.as_str())
    }
}

impl Display for PluginConstructionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::FailedToParseMetaData => write!(f, "Failed to parse meta data"),
            Self::FailedToParseWidgetConfig => write!(f, "Failed to parse widget config"),
            Self::FailedToCreateRuntime => write!(f, "Failed to create runtime"),
            Self::ConfigJsonIsNull => write!(f, "Config JSON is null"),
            Self::InvalidUtf8Config => write!(f, "Invalid UTF-8 config"),
            Self::FailedToParseConfig => write!(f, "Failed to parse config"),
            Self::Custom => write!(f, "Custom Error"),
        }
    }
}
