use smearor_model_mcp::RegisteredResource;

use crate::resources::SdkResourceFields;

impl SdkResourceFields for RegisteredResource {
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
