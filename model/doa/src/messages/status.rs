use serde::Deserialize;
use serde::Serialize;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::SharedMessage;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::generate_type_id;

use crate::DoaDirection;

pub const TOPIC_STATUS: &str = "service.doa.status";

/// Status message broadcast by the DoA service to all subscribers.
///
/// Contains the current DoA angle, calibrated angle, mapped compass direction,
/// hardware VAD flag, and USB device identifiers.
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DoaStatusMessage {
    /// Whether the ReSpeaker XVF3800 device is connected and active.
    pub connected: bool,
    /// Current DoA angle in degrees (0-359). Raw angle from the DSP, before rotation offset.
    pub angle: u16,
    /// Calibrated angle after applying `rotation_offset` from service config (0-359).
    /// This is the angle relative to the table's physical orientation.
    pub calibrated_angle: u16,
    /// Mapped table side based on `calibrated_angle`. Derived via `DoaDirection::from_angle`.
    pub direction: DoaDirection,
    /// Whether speech/voice activity is currently detected by the DSP.
    /// When `false`, the `angle` and `calibrated_angle` fields represent the
    /// last detected direction (held in the register during silence).
    /// When `true`, active speech is coming from the indicated direction.
    pub speech_detected: bool,
    /// Vendor ID of the connected device (0x2886 = Seeed Studio, 0x20b1 = XMOS).
    pub vendor_id: u16,
    /// Product ID of the connected device.
    pub product_id: u16,
    /// Timestamp of the last DoA reading (ISO 8601 or epoch seconds).
    pub last_updated: stabby::string::String,
}

impl TypedMessage for DoaStatusMessage {
    const TYPE_ID: u64 = generate_type_id("smearor_doa_model::DoaStatusMessage");
}

impl MessageTopic for DoaStatusMessage {
    fn topic() -> &'static str {
        TOPIC_STATUS
    }
}

impl SharedMessage for DoaStatusMessage {
    fn topic(&self) -> &'static str {
        TOPIC_STATUS
    }
}

#[cfg(test)]
mod tests {
    use super::DoaStatusMessage;
    use super::TOPIC_STATUS;
    use crate::DoaDirection;
    use smearor_swipe_launcher_plugin_api::MessageTopic;
    use smearor_swipe_launcher_plugin_api::SharedMessage;

    #[test]
    fn test_default_values() {
        let msg = DoaStatusMessage::default();
        assert!(!msg.connected);
        assert_eq!(msg.angle, 0);
        assert_eq!(msg.calibrated_angle, 0);
        assert_eq!(msg.direction, DoaDirection::North);
        assert!(!msg.speech_detected);
        assert_eq!(msg.vendor_id, 0);
        assert_eq!(msg.product_id, 0);
        assert_eq!(msg.last_updated.to_string(), "");
    }

    #[test]
    fn test_serde_round_trip() {
        let msg = DoaStatusMessage {
            connected: true,
            angle: 42,
            calibrated_angle: 132,
            direction: DoaDirection::East,
            speech_detected: true,
            vendor_id: 0x2886,
            product_id: 0x0021,
            last_updated: stabby::string::String::from("2025-01-01T00:00:00Z"),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: DoaStatusMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_serde_default_round_trip() {
        let msg = DoaStatusMessage::default();
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: DoaStatusMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
    }

    #[test]
    fn test_topic() {
        use smearor_swipe_launcher_plugin_api::MessageTopic;
        assert_eq!(<DoaStatusMessage as MessageTopic>::topic(), "service.doa.status");
        assert_eq!(TOPIC_STATUS, "service.doa.status");
        let msg = DoaStatusMessage::default();
        assert_eq!(SharedMessage::topic(&msg), "service.doa.status");
    }

    #[test]
    fn test_speech_detected_field_preserved() {
        let msg = DoaStatusMessage {
            speech_detected: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: DoaStatusMessage = serde_json::from_str(&json).unwrap();
        assert!(deserialized.speech_detected);
    }

    #[test]
    fn test_angle_held_during_silence() {
        let msg = DoaStatusMessage {
            angle: 180,
            calibrated_angle: 180,
            speech_detected: false,
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: DoaStatusMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.angle, 180);
        assert_eq!(deserialized.calibrated_angle, 180);
        assert!(!deserialized.speech_detected);
    }
}
