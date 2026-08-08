use crate::resources::creator::ResourceDefinitionCreator;

/// Resource listing all configured button widgets across all areas.
pub struct AreaButtonsResource;

impl ResourceDefinitionCreator for AreaButtonsResource {
    fn resource_uri() -> &'static str {
        "area://buttons"
    }
    fn resource_name() -> &'static str {
        "area_buttons"
    }
    fn resource_description() -> &'static str {
        "Lists all configured button widgets across all areas with their full action configuration (topics, payloads, state topics, icons, etc.)."
    }
    fn resource_mime_type() -> &'static str {
        "application/json"
    }
}
