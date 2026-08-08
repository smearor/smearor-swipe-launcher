use async_channel::Sender;

use crate::McpCommand;
use crate::resources::definition::ResourceDefinition;
use crate::resources::definition::ResourceFuture;
use crate::resources::definition::ResourceHandler;

/// Trait that lets a resource type generate its own `ResourceDefinition`.
pub trait ResourceDefinitionCreator {
    /// The URI that identifies this resource (e.g. `area://list`).
    fn resource_uri() -> &'static str;
    /// A short human-readable name for the resource.
    fn resource_name() -> &'static str;
    /// A human-readable description of what the resource provides.
    fn resource_description() -> &'static str;
    /// The MIME type of the content returned by the handler.
    fn resource_mime_type() -> &'static str;

    /// Create the full `ResourceDefinition` with URI, name, description, MIME type and handler.
    fn create_resource_definition() -> ResourceDefinition {
        ResourceDefinition::builder()
            .uri(Self::resource_uri())
            .name(Self::resource_name())
            .description(Self::resource_description())
            .mime_type(Self::resource_mime_type())
            .handler(Self::resource_handler())
            .build()
    }

    /// Build the default resource handler that reads the resource via the launcher core.
    fn resource_handler() -> ResourceHandler {
        Box::new(|sender: Sender<McpCommand>, _uri: String| -> ResourceFuture {
            Box::pin(async move {
                super::read_resource(sender, Self::resource_uri().to_string(), Self::resource_mime_type().to_string())
                    .await
                    .map(|output| output.contents)
            })
        })
    }
}
