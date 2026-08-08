/// The type of launcher instance, determining whether a GTK window is created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceType {
    /// A standard GTK launcher instance with a visible window.
    Gtk,
    /// A headless instance without a window (e.g. for MacroPad hardware devices).
    Headless,
    /// A web instance without a window, served via the embedded HTTP server.
    /// See `concepts/WEB_INSTANCE_CONCEPT.md`.
    Web,
}

impl InstanceType {
    /// Returns `true` if this instance type has a GTK window.
    pub fn has_window(self) -> bool {
        matches!(self, InstanceType::Gtk)
    }

    /// Returns the string representation used in config and MCP tools.
    pub fn as_str(self) -> &'static str {
        match self {
            InstanceType::Gtk => "gtk",
            InstanceType::Headless => "headless",
            InstanceType::Web => "web",
        }
    }
}

impl std::str::FromStr for InstanceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gtk" => Ok(InstanceType::Gtk),
            "headless" => Ok(InstanceType::Headless),
            "web" => Ok(InstanceType::Web),
            other => Err(format!("unknown instance type: {}", other)),
        }
    }
}

impl From<&smearor_model_instance_control::InstanceType> for InstanceType {
    fn from(value: &smearor_model_instance_control::InstanceType) -> Self {
        match value {
            smearor_model_instance_control::InstanceType::Gtk => InstanceType::Gtk,
            smearor_model_instance_control::InstanceType::Headless => InstanceType::Headless,
            smearor_model_instance_control::InstanceType::Web => InstanceType::Web,
        }
    }
}
