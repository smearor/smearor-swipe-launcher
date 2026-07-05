use serde::Deserialize;
use serde::Serialize;

/// A single HTTP header name/value pair for JSON serialization.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpHeader {
    /// Header name.
    pub name: String,
    /// Header value.
    pub value: String,
}

/// ABI-stable version of `HttpHeader`.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug)]
pub struct HttpHeaderStabby {
    /// Header name.
    pub name: stabby::string::String,
    /// Header value.
    pub value: stabby::string::String,
}

impl From<HttpHeader> for HttpHeaderStabby {
    fn from(value: HttpHeader) -> Self {
        Self {
            name: value.name.into(),
            value: value.value.into(),
        }
    }
}

impl From<HttpHeaderStabby> for HttpHeader {
    fn from(value: HttpHeaderStabby) -> Self {
        Self {
            name: value.name.to_string(),
            value: value.value.to_string(),
        }
    }
}
