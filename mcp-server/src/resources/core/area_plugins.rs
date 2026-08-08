use crate::resources::creator::ResourceDefinitionCreator;

/// Resource listing all plugins across all configured areas.
pub struct AreaPluginsResource;

impl ResourceDefinitionCreator for AreaPluginsResource {
    fn resource_uri() -> &'static str {
        "area://plugins"
    }
    fn resource_name() -> &'static str {
        "area_plugins"
    }
    fn resource_description() -> &'static str {
        "Lists all plugins across all configured areas with their IDs, library paths, and area assignments."
    }
    fn resource_mime_type() -> &'static str {
        "application/json"
    }
}
