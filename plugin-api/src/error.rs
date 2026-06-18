use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// Errors that can occur during plugin construction.
#[repr(u8)]
#[stabby::stabby]
#[derive(Debug)]
pub enum PluginConstructionError {
    FailedToParseMetaData,
    FailedToParseWidgetConfig,
    FailedToCreateRuntime,
    ConfigJsonIsNull,
    InvalidUtf8Config,
    FailedToParseConfig,
    Custom,
}

/// Wrapper around a plugin construction error with a descriptive message.
#[stabby::stabby]
#[derive(Debug)]
pub struct PluginConstructionErrorWrapper {
    pub error: PluginConstructionError,
    pub message: stabby::string::String,
}

impl PluginConstructionErrorWrapper {
    pub fn new(error: PluginConstructionError, message: stabby::string::String) -> Self {
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
