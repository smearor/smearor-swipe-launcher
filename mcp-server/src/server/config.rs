/// Configuration for the MCP server.
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    /// Address to bind the HTTP server to.
    pub bind_address: String,
    /// TCP port to listen on.
    pub port: u16,
    /// Optional bearer token required for all HTTP requests.
    pub auth_token: Option<String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8765,
            auth_token: None,
        }
    }
}
