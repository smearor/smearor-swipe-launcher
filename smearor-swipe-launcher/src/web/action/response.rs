use serde::Serialize;

/// Response body for the `/instances/{id}/{plugin_id}/{action}` endpoint.
#[derive(Serialize)]
pub struct ActionResponse {
    /// Whether the action was successfully processed.
    pub ok: bool,
    /// Human-readable status or error message.
    pub message: String,
    /// Optional updated widgets HTML, omitted from JSON when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widgets_html: Option<String>,
}
