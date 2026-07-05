use serde::Deserialize;
use serde::Serialize;

use crate::messages::header::HttpHeader;
use crate::messages::header::HttpHeaderStabby;
use crate::messages::method::HttpMethod;

/// Payload sent to the HTTP service to request an outbound HTTP call.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpRequestMessage {
    /// HTTP method to use.
    #[serde(default)]
    pub method: HttpMethod,
    /// Full URL to call. Must match the configured whitelist.
    pub url: String,
    /// Topic where the response should be published.
    pub response_topic: String,
    /// Optional JSON body for POST or PUT requests.
    #[serde(default)]
    pub body: Option<String>,
    /// Optional HTTP headers to send with the request.
    #[serde(default)]
    pub headers: Option<Vec<HttpHeader>>,
    /// Optional timeout in milliseconds. Defaults to the service configuration.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// ABI-stable version of `HttpRequestMessage`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug)]
pub struct HttpRequestMessageStabby {
    /// HTTP method to use.
    pub method: HttpMethod,
    /// Full URL to call. Must match the configured whitelist.
    pub url: stabby::string::String,
    /// Topic where the response should be published.
    pub response_topic: stabby::string::String,
    /// Optional JSON body for POST or PUT requests.
    pub body: stabby::option::Option<stabby::string::String>,
    /// Optional HTTP headers to send with the request.
    pub headers: stabby::option::Option<stabby::vec::Vec<HttpHeaderStabby>>,
    /// Optional timeout in milliseconds. Defaults to the service configuration.
    pub timeout_ms: stabby::option::Option<u64>,
}

impl From<HttpRequestMessage> for HttpRequestMessageStabby {
    fn from(value: HttpRequestMessage) -> Self {
        Self {
            method: value.method,
            url: value.url.into(),
            response_topic: value.response_topic.into(),
            body: value.body.map(Into::into).into(),
            headers: value
                .headers
                .map(|headers| headers.into_iter().map(Into::into).collect::<stabby::vec::Vec<HttpHeaderStabby>>())
                .into(),
            timeout_ms: value.timeout_ms.into(),
        }
    }
}

impl From<HttpRequestMessageStabby> for HttpRequestMessage {
    fn from(value: HttpRequestMessageStabby) -> Self {
        Self {
            method: value.method,
            url: value.url.to_string(),
            response_topic: value.response_topic.to_string(),
            body: {
                let opt: Option<stabby::string::String> = value.body.into();
                opt.map(Into::into)
            },
            headers: {
                let opt: Option<stabby::vec::Vec<HttpHeaderStabby>> = value.headers.into();
                opt.map(|headers| headers.into_iter().map(Into::into).collect())
            },
            timeout_ms: {
                let opt: Option<u64> = value.timeout_ms.into();
                opt
            },
        }
    }
}
