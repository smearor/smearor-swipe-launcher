use serde::Deserialize;
use serde::Serialize;

/// Supported HTTP methods for outbound requests.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum HttpMethod {
    /// Retrieve a resource.
    #[default]
    Get,
    /// Submit data to a resource.
    Post,
    /// Update a resource.
    Put,
    /// Delete a resource.
    Delete,
}
