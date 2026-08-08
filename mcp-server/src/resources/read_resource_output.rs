/// Output of a resource read operation.
pub struct ReadResourceOutput {
    /// The text content of the resource.
    pub contents: String,
    /// The MIME type of the resource content.
    pub mime_type: String,
}
