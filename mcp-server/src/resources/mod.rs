//! MCP resource definitions and invocation helpers.

mod core;
mod creator;
mod definition;
mod into_sdk_resource;
mod read_resource_output;
mod registered;
mod resolver;
mod resource_content;
mod result;

pub use definition::ResourceDefinition;
pub use definition::ResourceFuture;
pub use definition::ResourceHandler;
pub use into_sdk_resource::IntoSdkResource;
pub use into_sdk_resource::SdkResourceFields;
pub use read_resource_output::ReadResourceOutput;
pub use resolver::ResourceResolver;
pub use resource_content::ReadResourceResult;
pub use resource_content::ResourceContent;
pub use result::ResourceResult;
