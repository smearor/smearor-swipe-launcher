use serde::Deserialize;
use serde::Serialize;

/// The type of launcher instance, determining whether a GTK window is created.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceType {
    /// A standard GTK launcher instance with a visible window.
    #[default]
    Gtk,
    /// A headless instance without a window (e.g. for MacroPad hardware devices).
    Headless,
    /// A web instance without a window, served via the embedded HTTP server.
    Web,
}

impl InstanceType {
    /// Returns the string representation used in config and MCP tools.
    pub fn as_str(&self) -> &'static str {
        match self {
            InstanceType::Gtk => "gtk",
            InstanceType::Headless => "headless",
            InstanceType::Web => "web",
        }
    }

    /// Parse from a string representation.
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "gtk" => Ok(InstanceType::Gtk),
            "headless" => Ok(InstanceType::Headless),
            "web" => Ok(InstanceType::Web),
            other => Err(format!("unknown instance type: {}", other)),
        }
    }
}
