use serde::Deserialize;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct AudioServiceConfig {
    /// Volume change step as a ratio (e.g. 0.05 for 5%)
    #[builder(default = 0.05)]
    pub(crate) volume_step: f32,
    /// Maximum allowed volume ratio (e.g. 1.5 for 150% overdrive)
    #[builder(default = 1.5)]
    pub(crate) max_volume: f32,
    /// Whether to enable MCP tool registration for this service.
    #[builder(default = true)]
    pub mcp_enabled: bool,
    /// Whether to enable DoA VAD-triggered audio ducking.
    /// When enabled, the service listens for `DoaStatusMessage` and ducks
    /// the master volume while speech is detected by the ReSpeaker XVF3800.
    #[builder(default = false)]
    pub ducking_enabled: bool,
    /// Target volume ratio during ducking (0.0–1.0, e.g. 0.2 for 20%).
    #[builder(default = 0.2)]
    pub ducking_volume: f32,
    /// Grace period in milliseconds after VAD falling edge before restoring volume.
    #[builder(default = 500)]
    pub ducking_grace_period_ms: u64,
    /// Minimum continuous VAD activity in milliseconds before ducking is triggered.
    /// Prevents false triggers from impulsive environmental noises.
    /// Set to 0 for instant ducking on rising edge.
    #[builder(default = 100)]
    pub min_speech_duration_ms: u64,
    /// Duration of the linear fade ramp for volume restore in milliseconds.
    /// A smooth ramp prevents abrupt volume jumps. Set to 0 for instant restore.
    #[builder(default = 500)]
    pub fade_ramp_ms: u64,
    /// Whether to duck notification/system sounds in addition to media.
    /// When false, only music/media streams are ducked.
    #[builder(default = false)]
    pub duck_notification_sounds: bool,
    /// Whether to duck media during Voice Assistant TTS playback.
    /// When false, ducking is suppressed while TTS is speaking.
    #[builder(default = true)]
    pub duck_during_tts: bool,
}

impl Default for AudioServiceConfig {
    fn default() -> Self {
        Self {
            volume_step: 0.05,
            max_volume: 1.5,
            mcp_enabled: true,
            ducking_enabled: false,
            ducking_volume: 0.2,
            ducking_grace_period_ms: 500,
            min_speech_duration_ms: 100,
            fade_ramp_ms: 500,
            duck_notification_sounds: false,
            duck_during_tts: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AudioServiceConfig;

    #[test]
    fn test_default_values() {
        let config = AudioServiceConfig::default();
        assert_eq!(config.volume_step, 0.05);
        assert_eq!(config.max_volume, 1.5);
        assert!(config.mcp_enabled);
        assert!(!config.ducking_enabled);
        assert_eq!(config.ducking_volume, 0.2);
        assert_eq!(config.ducking_grace_period_ms, 500);
        assert_eq!(config.min_speech_duration_ms, 100);
        assert_eq!(config.fade_ramp_ms, 500);
        assert!(!config.duck_notification_sounds);
        assert!(config.duck_during_tts);
    }

    #[test]
    fn test_serde_deserialize() {
        let json = serde_json::json!({
            "volume_step": 0.1,
            "max_volume": 1.0,
            "mcp_enabled": false,
            "ducking_enabled": true,
            "ducking_volume": 0.15,
            "ducking_grace_period_ms": 300
        });
        let config: AudioServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.volume_step, 0.1);
        assert_eq!(config.max_volume, 1.0);
        assert!(!config.mcp_enabled);
        assert!(config.ducking_enabled);
        assert_eq!(config.ducking_volume, 0.15);
        assert_eq!(config.ducking_grace_period_ms, 300);
    }

    #[test]
    fn test_partial_json_uses_defaults() {
        let json = serde_json::json!({
            "ducking_enabled": true,
            "ducking_volume": 0.3
        });
        let config: AudioServiceConfig = serde_json::from_value(json).unwrap();
        assert!(config.ducking_enabled);
        assert_eq!(config.ducking_volume, 0.3);
        assert_eq!(config.ducking_grace_period_ms, 500);
        assert_eq!(config.min_speech_duration_ms, 100);
        assert_eq!(config.fade_ramp_ms, 500);
        assert!(config.mcp_enabled);
    }

    #[test]
    fn test_empty_json_uses_defaults() {
        let json = serde_json::json!({});
        let config: AudioServiceConfig = serde_json::from_value(json).unwrap();
        assert!(!config.ducking_enabled);
        assert_eq!(config.ducking_volume, 0.2);
        assert_eq!(config.ducking_grace_period_ms, 500);
        assert_eq!(config.min_speech_duration_ms, 100);
        assert_eq!(config.fade_ramp_ms, 500);
        assert!(!config.duck_notification_sounds);
        assert!(config.duck_during_tts);
    }

    #[test]
    fn test_vad_rising_edge_for_ducking() {
        use smearor_doa_model::VadTransition;
        use smearor_doa_model::classify_vad_transition;
        assert_eq!(classify_vad_transition(false, true), VadTransition::RisingEdge);
    }

    #[test]
    fn test_vad_falling_edge_for_ducking() {
        use smearor_doa_model::VadTransition;
        use smearor_doa_model::classify_vad_transition;
        assert_eq!(classify_vad_transition(true, false), VadTransition::FallingEdge);
    }

    #[test]
    fn test_vad_should_duck_after_min_duration() {
        use smearor_doa_model::should_activate_after_min_duration;
        let onset = std::time::Instant::now();
        let now = onset + std::time::Duration::from_millis(150);
        assert!(should_activate_after_min_duration(Some(onset), 100, now));
    }

    #[test]
    fn test_vad_should_not_duck_before_min_duration() {
        use smearor_doa_model::should_activate_after_min_duration;
        let onset = std::time::Instant::now();
        let now = onset + std::time::Duration::from_millis(50);
        assert!(!should_activate_after_min_duration(Some(onset), 100, now));
    }

    #[test]
    fn test_vad_instant_duck_with_zero_min_duration() {
        use smearor_doa_model::should_activate_after_min_duration;
        let onset = std::time::Instant::now();
        assert!(should_activate_after_min_duration(Some(onset), 0, onset));
    }
}
