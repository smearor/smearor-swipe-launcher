use smearor_model_mcp::UnknownToolError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP tools registered by the HTTP service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpMcpTools {
    /// Execute an HTTP request and return the response.
    HttpRequest,
}

impl AsRef<str> for HttpMcpTools {
    fn as_ref(&self) -> &str {
        match self {
            Self::HttpRequest => "http_request",
        }
    }
}

impl FromStr for HttpMcpTools {
    type Err = UnknownToolError;

    fn from_str(tool: &str) -> Result<Self, Self::Err> {
        match tool {
            "http_request" => Ok(Self::HttpRequest),
            _ => Err(UnknownToolError::new(tool)),
        }
    }
}

impl Display for HttpMcpTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
