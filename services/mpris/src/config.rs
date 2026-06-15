use serde::Deserialize;
use typed_builder::TypedBuilder;

/// Configuration for the MPRIS service plugin.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct MprisServiceConfig {
    /// D-Bus address override (optional)
    #[builder(default, setter(into))]
    pub(crate) dbus_address: Option<String>,
}

impl Default for MprisServiceConfig {
    fn default() -> Self {
        Self { dbus_address: None }
    }
}
