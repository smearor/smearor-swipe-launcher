use crate::resources::creator::ResourceDefinitionCreator;

/// Resource listing all configured launcher areas.
pub struct AreaListResource;

impl ResourceDefinitionCreator for AreaListResource {
    fn resource_uri() -> &'static str {
        "area://list"
    }
    fn resource_name() -> &'static str {
        "area_list"
    }
    fn resource_description() -> &'static str {
        "Current list of all configured Smearor launcher areas (Bereiche) with their area_id, visibility status, and position. Use this resource when the user asks for areas, 'Bereiche', or 'Areas' in the launcher."
    }
    fn resource_mime_type() -> &'static str {
        "application/json"
    }
}
