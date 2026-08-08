use serde::Deserialize;
use serde::Serialize;
use typed_builder::TypedBuilder;

/// Configuration for the embedded web server.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TypedBuilder)]
pub struct WebServerConfig {
    /// TCP port to listen on.
    pub port: u16,
    /// Whether the web server is enabled.
    pub enabled: bool,
    /// Address to bind to.
    pub bind_address: String,
    /// Optional bearer token for authentication.
    pub auth_token: Option<String>,
    /// Allowed CORS origins.
    pub allowed_origins: Vec<String>,
}
