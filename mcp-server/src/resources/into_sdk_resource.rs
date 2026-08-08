use rust_mcp_sdk::schema::Resource;

/// Trait for converting resource-like types into the SDK `Resource` type.
pub trait IntoSdkResource {
    /// Convert into the SDK `Resource` representation.
    fn into_sdk_resource(&self) -> Resource;
}

/// Fields shared by all resource-like types that can be converted to an SDK `Resource`.
pub trait SdkResourceFields {
    /// The URI that identifies this resource.
    fn uri(&self) -> &str;
    /// A short human-readable name for the resource.
    fn name(&self) -> &str;
    /// A human-readable description of what the resource provides.
    fn description(&self) -> &str;
    /// The MIME type of the resource content.
    fn mime_type(&self) -> &str;
}

impl<T: SdkResourceFields> IntoSdkResource for T {
    fn into_sdk_resource(&self) -> Resource {
        Resource {
            uri: self.uri().to_string(),
            name: self.name().to_string(),
            description: Some(self.description().to_string()),
            mime_type: Some(self.mime_type().to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
            title: None,
        }
    }
}
