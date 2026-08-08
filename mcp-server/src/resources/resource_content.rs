use serde::Serialize;

/// A single resource content entry returned by the MCP `read_resource` response.
#[derive(Serialize)]
pub struct ResourceContent {
    /// The URI of the resource that was read.
    pub uri: String,
    /// The MIME type of the resource content.
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// The text content of the resource.
    pub text: String,
}

/// The `contents` payload of a `read_resource` JSON-RPC response.
#[derive(Serialize)]
pub struct ReadResourceResult {
    /// The list of resource content entries.
    pub contents: Vec<ResourceContent>,
}
