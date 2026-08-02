use serde::Deserialize;
use serde::Serialize;

/// Information about an audio device.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioDevice {
    /// Unique device identifier
    pub id: u32,
    /// Human-readable device name
    pub name: stabby::string::String,
    /// Whether this is the default/active device
    pub is_default: bool,
}
