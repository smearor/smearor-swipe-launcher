use std::time::Instant;

/// VAD transition classification based on previous and current speech detection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadTransition {
    /// Speech started (false → true).
    RisingEdge,
    /// Speech continues (true → true).
    ContinuousSpeech,
    /// Speech stopped (true → false).
    FallingEdge,
    /// No speech before or after (false → false).
    NoChange,
}

/// Previous and current speech detection state used to classify a VAD transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VadSpeakingState {
    /// Whether speech was detected in the previous reading.
    pub was_speaking: bool,
    /// Whether speech is detected in the current reading.
    pub is_speaking: bool,
}

impl From<VadSpeakingState> for VadTransition {
    fn from(state: VadSpeakingState) -> Self {
        match (state.was_speaking, state.is_speaking) {
            (false, true) => VadTransition::RisingEdge,
            (true, true) => VadTransition::ContinuousSpeech,
            (true, false) => VadTransition::FallingEdge,
            (false, false) => VadTransition::NoChange,
        }
    }
}

/// Classifies the VAD transition from the previous and current speech detection state.
pub fn classify_vad_transition(was_speaking: bool, is_speaking: bool) -> VadTransition {
    VadTransition::from(VadSpeakingState { was_speaking, is_speaking })
}

/// Checks whether enough time has elapsed since the VAD onset to trigger activation.
///
/// Returns `true` if the elapsed time since `onset` is at least `min_speech_duration_ms`.
/// Returns `false` if `onset` is `None` (no rising edge recorded) or the duration
/// has not yet elapsed.
pub fn should_activate_after_min_duration(onset: Option<Instant>, min_speech_duration_ms: u64, now: Instant) -> bool {
    match onset {
        Some(onset_time) => now.duration_since(onset_time).as_millis() as u64 >= min_speech_duration_ms,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_rising_edge() {
        assert_eq!(classify_vad_transition(false, true), VadTransition::RisingEdge);
    }

    #[test]
    fn test_falling_edge() {
        assert_eq!(classify_vad_transition(true, false), VadTransition::FallingEdge);
    }

    #[test]
    fn test_continuous_speech() {
        assert_eq!(classify_vad_transition(true, true), VadTransition::ContinuousSpeech);
    }

    #[test]
    fn test_no_change() {
        assert_eq!(classify_vad_transition(false, false), VadTransition::NoChange);
    }

    #[test]
    fn test_from_vad_speaking_state_rising_edge() {
        let state = VadSpeakingState {
            was_speaking: false,
            is_speaking: true,
        };
        assert_eq!(VadTransition::from(state), VadTransition::RisingEdge);
    }

    #[test]
    fn test_from_vad_speaking_state_falling_edge() {
        let state = VadSpeakingState {
            was_speaking: true,
            is_speaking: false,
        };
        assert_eq!(VadTransition::from(state), VadTransition::FallingEdge);
    }

    #[test]
    fn test_from_vad_speaking_state_continuous_speech() {
        let state = VadSpeakingState {
            was_speaking: true,
            is_speaking: true,
        };
        assert_eq!(VadTransition::from(state), VadTransition::ContinuousSpeech);
    }

    #[test]
    fn test_from_vad_speaking_state_no_change() {
        let state = VadSpeakingState {
            was_speaking: false,
            is_speaking: false,
        };
        assert_eq!(VadTransition::from(state), VadTransition::NoChange);
    }

    #[test]
    fn test_should_activate_after_min_duration_elapsed() {
        let onset = Instant::now();
        let now = onset + std::time::Duration::from_millis(150);
        assert!(should_activate_after_min_duration(Some(onset), 100, now));
    }

    #[test]
    fn test_should_activate_after_min_duration_not_elapsed() {
        let onset = Instant::now();
        let now = onset + std::time::Duration::from_millis(50);
        assert!(!should_activate_after_min_duration(Some(onset), 100, now));
    }

    #[test]
    fn test_should_activate_after_min_duration_exact_boundary() {
        let onset = Instant::now();
        let now = onset + std::time::Duration::from_millis(100);
        assert!(should_activate_after_min_duration(Some(onset), 100, now));
    }

    #[test]
    fn test_should_activate_after_min_duration_no_onset() {
        let now = Instant::now();
        assert!(!should_activate_after_min_duration(None, 100, now));
    }

    #[test]
    fn test_should_activate_with_zero_min_duration() {
        let onset = Instant::now();
        let now = onset;
        assert!(should_activate_after_min_duration(Some(onset), 0, now));
    }
}
