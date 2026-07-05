use serde::Deserialize;

/// Configuration for the HTTP service.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HttpServiceConfig {
    /// List of URL patterns that may be called. Wildcards `*` are supported.
    pub allowed_urls: Vec<String>,
    /// Default request timeout in milliseconds.
    pub default_timeout_ms: u64,
    /// Maximum allowed response body size in bytes.
    pub max_response_bytes: usize,
}

impl Default for HttpServiceConfig {
    fn default() -> Self {
        Self {
            allowed_urls: Vec::new(),
            default_timeout_ms: 10000,
            max_response_bytes: 1024 * 1024,
        }
    }
}
