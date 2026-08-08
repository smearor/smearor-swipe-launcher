use crate::resources::creator::ResourceDefinitionCreator;

/// Resource listing all loaded plugins.
pub struct PluginListResource;

impl ResourceDefinitionCreator for PluginListResource {
    fn resource_uri() -> &'static str {
        "plugin://list"
    }
    fn resource_name() -> &'static str {
        "plugin_list"
    }
    fn resource_description() -> &'static str {
        "Lists all loaded plugins (services and widgets) with their IDs, library paths, and type."
    }
    fn resource_mime_type() -> &'static str {
        "application/json"
    }
}
