use smearor_doa_model::DoaDirection;
use smearor_doa_model::DoaDirectionResponse;

/// Shared state between the async control loop and the MCP tool handler.
/// Updated only by the async loop — never accessed from the USB reader thread.
#[derive(Clone)]
pub struct DoaSharedState {
    /// Whether the ReSpeaker XVF3800 device is connected and active.
    pub connected: bool,
    /// Current DoA angle in degrees (0-359). Raw angle from the DSP, before rotation offset.
    pub angle: u16,
    /// Calibrated angle after applying `rotation_offset` from service config (0-359).
    pub calibrated_angle: u16,
    /// The service's configured rotation offset in degrees.
    pub rotation_offset: i16,
    /// Whether speech/voice activity is currently detected by the DSP.
    pub speech_detected: bool,
    /// Vendor ID of the connected device.
    pub vendor_id: u16,
    /// Product ID of the connected device.
    pub product_id: u16,
    /// Timestamp of the last DoA reading.
    pub last_updated: String,
    /// Whether DoA polling is currently paused.
    pub paused: bool,
}

impl Default for DoaSharedState {
    fn default() -> Self {
        Self {
            connected: false,
            angle: 0,
            calibrated_angle: 0,
            rotation_offset: 0,
            speech_detected: false,
            vendor_id: 0,
            product_id: 0,
            last_updated: String::new(),
            paused: false,
        }
    }
}

impl From<DoaSharedState> for DoaDirectionResponse {
    fn from(state: DoaSharedState) -> Self {
        Self {
            connected: state.connected,
            angle: state.angle,
            calibrated_angle: state.calibrated_angle,
            rotation_offset: state.rotation_offset,
            direction: DoaDirection::from_angle(state.calibrated_angle),
            speech_detected: state.speech_detected,
            vendor_id: state.vendor_id,
            product_id: state.product_id,
            last_updated: state.last_updated,
            paused: state.paused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DoaDirectionResponse;
    use super::DoaSharedState;
    use smearor_doa_model::DoaDirection;

    #[test]
    fn test_default() {
        let state = DoaSharedState::default();
        assert!(!state.connected);
        assert_eq!(state.angle, 0);
        assert_eq!(state.calibrated_angle, 0);
        assert_eq!(state.rotation_offset, 0);
        assert!(!state.speech_detected);
        assert_eq!(state.vendor_id, 0);
        assert_eq!(state.product_id, 0);
        assert_eq!(state.last_updated, "");
        assert!(!state.paused);
    }

    #[test]
    fn test_from_state_to_response_north() {
        let state = DoaSharedState {
            connected: true,
            angle: 10,
            calibrated_angle: 10,
            rotation_offset: 0,
            speech_detected: true,
            vendor_id: 0x2886,
            product_id: 0x0021,
            last_updated: "2025-01-01".to_string(),
            paused: false,
        };
        let response = DoaDirectionResponse::from(state);
        assert!(response.connected);
        assert_eq!(response.angle, 10);
        assert_eq!(response.calibrated_angle, 10);
        assert_eq!(response.rotation_offset, 0);
        assert_eq!(response.direction, DoaDirection::North);
        assert!(response.speech_detected);
    }

    #[test]
    fn test_from_state_to_response_east() {
        let state = DoaSharedState {
            calibrated_angle: 90,
            ..Default::default()
        };
        let response = DoaDirectionResponse::from(state);
        assert_eq!(response.direction, DoaDirection::East);
    }

    #[test]
    fn test_from_state_to_response_south() {
        let state = DoaSharedState {
            calibrated_angle: 180,
            ..Default::default()
        };
        let response = DoaDirectionResponse::from(state);
        assert_eq!(response.direction, DoaDirection::South);
    }

    #[test]
    fn test_from_state_to_response_west() {
        let state = DoaSharedState {
            calibrated_angle: 270,
            ..Default::default()
        };
        let response = DoaDirectionResponse::from(state);
        assert_eq!(response.direction, DoaDirection::West);
    }

    #[test]
    fn test_from_state_to_response_vendor_product_hex() {
        let state = DoaSharedState {
            vendor_id: 0x2886,
            product_id: 0x0021,
            ..Default::default()
        };
        let response = DoaDirectionResponse::from(state);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""0x2886""#));
        assert!(json.contains(r#""0x0021""#));
    }

    #[test]
    fn test_response_json_contains_all_fields() {
        let state = DoaSharedState {
            connected: true,
            angle: 45,
            calibrated_angle: 135,
            rotation_offset: 90,
            speech_detected: true,
            vendor_id: 0x2886,
            product_id: 0x0021,
            last_updated: "2025-01-01T00:00:00Z".to_string(),
            paused: false,
        };
        let response = DoaDirectionResponse::from(state);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""connected":true"#));
        assert!(json.contains(r#""angle":45"#));
        assert!(json.contains(r#""calibrated_angle":135"#));
        assert!(json.contains(r#""rotation_offset":90"#));
        assert!(json.contains(r#""direction":"South""#));
        assert!(json.contains(r#""speech_detected":true"#));
        assert!(json.contains(r#""paused":false"#));
        assert!(json.contains(r#""last_updated":"2025-01-01T00:00:00Z""#));
    }

    #[test]
    fn test_response_disconnected_state() {
        let state = DoaSharedState::default();
        let response = DoaDirectionResponse::from(state);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""connected":false"#));
        assert!(json.contains(r#""direction":"North""#));
        assert!(json.contains(r#""speech_detected":false"#));
    }

    #[test]
    fn test_response_paused_state() {
        let state = DoaSharedState {
            paused: true,
            connected: true,
            angle: 90,
            calibrated_angle: 90,
            ..Default::default()
        };
        let response = DoaDirectionResponse::from(state);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""paused":true"#));
        assert!(json.contains(r#""connected":true"#));
    }
}
