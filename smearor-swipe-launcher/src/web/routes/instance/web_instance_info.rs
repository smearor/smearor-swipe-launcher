use serde::Serialize;

/// JSON response item for the `/instances` list endpoint.
#[derive(Serialize)]
pub struct WebInstanceInfo {
    /// The unique instance identifier.
    pub instance_id: String,
    /// The instance type as a string (e.g. `"web"`, `"gtk"`).
    pub instance_type: String,
}
