use serde::Serialize;

use crate::DoaDirection;

/// JSON response payload for the `doa_get_direction` MCP tool and the `doa://status` resource.
///
/// Wraps the live DoA status fields with the service's `rotation_offset` for
/// client-side calibration context. Unlike `DoaStatusMessage` (which is an
/// FFI type with `#[stabby::stabby]`), this struct is a plain Serde type
/// intended for JSON serialization to MCP clients.
#[derive(Debug, Clone, Serialize)]
pub struct DoaDirectionResponse {
    /// Whether the ReSpeaker XVF3800 device is connected and active.
    pub connected: bool,
    /// Current DoA angle in degrees (0-359). Raw angle from the DSP, before rotation offset.
    pub angle: u16,
    /// Calibrated angle after applying `rotation_offset` from service config (0-359).
    pub calibrated_angle: u16,
    /// The service's configured rotation offset in degrees.
    pub rotation_offset: i16,
    /// Mapped compass direction based on `calibrated_angle`.
    pub direction: DoaDirection,
    /// Whether speech/voice activity is currently detected by the DSP.
    pub speech_detected: bool,
    /// Vendor ID of the connected device as a hex string (e.g. "0x2886").
    #[serde(serialize_with = "serialize_hex_u16")]
    pub vendor_id: u16,
    /// Product ID of the connected device as a hex string (e.g. "0x0001").
    #[serde(serialize_with = "serialize_hex_u16")]
    pub product_id: u16,
    /// Timestamp of the last DoA reading.
    pub last_updated: String,
}

fn serialize_hex_u16<S: serde::Serializer>(value: &u16, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&format!("{:#06x}", value))
}
