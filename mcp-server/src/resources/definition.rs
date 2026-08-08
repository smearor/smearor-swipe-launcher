use async_channel::Sender;
use typed_builder::TypedBuilder;

use crate::McpCommand;
use crate::resources::core::AreaButtonsResource;
use crate::resources::core::AreaListResource;
use crate::resources::core::AreaPluginsResource;
use crate::resources::core::PluginListResource;
use crate::resources::creator::ResourceDefinitionCreator;
use crate::resources::into_sdk_resource::SdkResourceFields;

use crate::resources::result::ResourceResult;

/// Resource handler signature.
pub type ResourceHandler = Box<dyn Fn(Sender<McpCommand>, String) -> ResourceFuture + Send + Sync>;

/// Future returned by a resource handler.
pub type ResourceFuture = std::pin::Pin<Box<dyn Future<Output = ResourceResult> + Send>>;

/// Built-in resource definition exposed by the MCP server.
#[derive(TypedBuilder)]
pub struct ResourceDefinition {
    /// The URI that identifies this resource (e.g. `area://list`).
    #[builder(setter(into))]
    pub uri: String,
    /// A short human-readable name for the resource.
    #[builder(setter(into))]
    pub name: String,
    /// A human-readable description of what the resource provides.
    #[builder(setter(into))]
    pub description: String,
    /// The MIME type of the content returned by the handler (e.g. `application/json`).
    #[builder(setter(into))]
    pub mime_type: String,
    /// The handler invoked when the resource is read, returning the content as a string.
    pub handler: ResourceHandler,
}

impl ResourceDefinition {
    /// Build the list of core resources available from the MVP.
    pub fn core_resources() -> Vec<ResourceDefinition> {
        vec![
            AreaListResource::create_resource_definition(),
            AreaPluginsResource::create_resource_definition(),
            AreaButtonsResource::create_resource_definition(),
            PluginListResource::create_resource_definition(),
        ]
    }
}

impl SdkResourceFields for ResourceDefinition {
    fn uri(&self) -> &str {
        &self.uri
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn mime_type(&self) -> &str {
        &self.mime_type
    }
}
