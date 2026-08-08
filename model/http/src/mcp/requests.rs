use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `http_request` MCP tool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct HttpRequestArgs {
    /// The URL to request
    pub url: String,
    /// HTTP method: "GET", "POST", "PUT", or "DELETE" (default: "GET")
    pub method: Option<String>,
    /// Request body for POST/PUT
    pub body: Option<String>,
    /// Request timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

/// Response body for the `http_request` MCP tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpRequestResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response body text
    pub body: String,
}
