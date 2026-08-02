use crate::DimmingPhase;
use std::time::Duration;
use std::time::Instant;

/// Per-device dimming state.
#[derive(Debug)]
pub struct DimmingState {
    /// Whether auto-dimming is enabled for this device.
    pub enabled: bool,
    /// Target brightness when active (0-100).
    pub target_brightness: u8,
    /// Dimmed brightness when idle (0-100).
    pub dim_brightness: u8,
    /// Idle timeout before dimming starts.
    pub idle_timeout: Duration,
    /// Fade step interval.
    pub fade_step_duration: Duration,
    /// Brightness change per fade step when dimming down.
    pub fade_step_percent: u8,
    /// Brightness change per fade step when restoring brightness.
    pub fade_up_step_percent: u8,
    /// Current brightness (may differ from target during fading).
    pub current_brightness: u8,
    /// Last activity timestamp.
    pub last_activity: Instant,
    /// Current dimming phase.
    pub phase: DimmingPhase,
}

impl DimmingState {
    /// Compute the sleep duration until the next dimming timer tick.
    pub fn timer_duration(&self) -> Duration {
        match self.phase {
            DimmingPhase::Active => {
                let elapsed = self.last_activity.elapsed();
                if elapsed >= self.idle_timeout {
                    Duration::ZERO
                } else {
                    self.idle_timeout - elapsed
                }
            }
            DimmingPhase::FadingDown | DimmingPhase::FadingUp => self.fade_step_duration,
            DimmingPhase::Dimmed => Duration::from_secs(86400 * 365),
        }
    }
}
