use serde::Deserialize;
use serde::Serialize;

/// Result status of an HTTP call.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum HttpResponseStatus {
    /// The call succeeded and the body is available.
    #[default]
    Success,
    /// The URL was not allowed by the whitelist.
    Forbidden,
    /// The HTTP call failed (network error, timeout, invalid response).
    Error,
}

/// Payload published by the HTTP service after a request completes.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HttpResponseMessage {
    /// Final status of the request.
    pub status: HttpResponseStatus,
    /// HTTP status code returned by the server, when available.
    #[serde(default)]
    pub status_code: Option<u16>,
    /// Response body as a JSON string. Empty when the call failed or was forbidden.
    #[serde(default)]
    pub body: String,
    /// Human-readable error message when the status is not success.
    #[serde(default)]
    pub error_message: Option<String>,
    /// Original request URL, echoed back for correlation.
    pub url: String,
}

/// ABI-stable version of `HttpResponseMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default)]
pub struct HttpResponseMessageStabby {
    /// Final status of the request.
    pub status: HttpResponseStatus,
    /// HTTP status code returned by the server, when available.
    pub status_code: stabby::option::Option<u16>,
    /// Response body as a JSON string. Empty when the call failed or was forbidden.
    pub body: stabby::string::String,
    /// Human-readable error message when the status is not success.
    pub error_message: stabby::option::Option<stabby::string::String>,
    /// Original request URL, echoed back for correlation.
    pub url: stabby::string::String,
}

impl From<HttpResponseMessage> for HttpResponseMessageStabby {
    fn from(value: HttpResponseMessage) -> Self {
        Self {
            status: value.status,
            status_code: value.status_code.into(),
            body: value.body.into(),
            error_message: value.error_message.map(Into::into).into(),
            url: value.url.into(),
        }
    }
}

impl From<HttpResponseMessageStabby> for HttpResponseMessage {
    fn from(value: HttpResponseMessageStabby) -> Self {
        Self {
            status: value.status,
            status_code: {
                let opt: Option<u16> = value.status_code.into();
                opt
            },
            body: value.body.to_string(),
            error_message: {
                let opt: Option<stabby::string::String> = value.error_message.into();
                opt.map(Into::into)
            },
            url: value.url.to_string(),
        }
    }
}
