use serde::Deserialize;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct AppLauncherServiceConfig {
    #[builder(default, setter(into))]
    pub(crate) smearor_wrot_path: Option<String>,
    #[builder(default, setter(into))]
    pub(crate) rotation: Option<f32>,
    /// Whether to enable MCP tool registration for this service.
    #[builder(default = true)]
    pub mcp_enabled: bool,
}

impl Default for AppLauncherServiceConfig {
    fn default() -> Self {
        Self {
            smearor_wrot_path: None,
            rotation: None,
            mcp_enabled: true,
        }
    }
}
