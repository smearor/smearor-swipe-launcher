use serde::Serialize;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;

/// Fallback JSON response when an FfiEnvelope payload is not a string.
#[derive(Default, Serialize)]
pub struct NonStringPayload {
    /// The type ID of the payload.
    pub type_id: u64,
    /// Human-readable note indicating the payload is not a string.
    pub note: &'static str,
}

impl NonStringPayload {
    /// Create a new `NonStringPayload` with the given type ID and the default note.
    pub fn new(type_id: u64) -> Self {
        Self {
            type_id,
            note: "non-string payload",
        }
    }
}

/// Generate a simple unique ID for correlation.
pub fn uuid_v4_simple() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{}{}", now.as_millis(), now.subsec_nanos())
}

/// Extract a JSON-serializable payload string from an FfiEnvelope.
///
/// For String payloads, returns the string directly.
/// For other payload types, returns a JSON object with type_id for the client
/// to interpret.
pub fn extract_payload_as_json(envelope: &FfiEnvelope) -> String {
    let string_type_id = smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String");

    if envelope.type_id == string_type_id && !envelope.payload.is_null() {
        if let Some(payload) = unsafe { (envelope.payload as *const String).as_ref() } {
            return payload.clone();
        }
    }

    serde_json::to_string(&NonStringPayload::new(envelope.type_id)).unwrap_or_default()
}
