use std::time::Duration;
use std::time::Instant;

/// Whether the countdown is running, paused, idle, or finished.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CountdownStatus {
    /// Countdown has not been started yet.
    #[default]
    Idle,
    /// Countdown is actively counting down.
    Running,
    /// Countdown was running and is now paused.
    Paused,
    /// Countdown reached zero.
    Finished,
}

/// State of the Countdown widget.
#[derive(Clone, Debug)]
pub struct CountdownState {
    /// Whether the countdown is running, paused, or idle.
    pub status: CountdownStatus,
    /// Target duration to count down from.
    pub target: Duration,
    /// Remaining time.
    pub remaining: Duration,
    /// Instant when the current run segment started.
    pub last_start: Option<Instant>,
}

impl Default for CountdownState {
    fn default() -> Self {
        Self {
            status: CountdownStatus::Idle,
            target: Duration::ZERO,
            remaining: Duration::ZERO,
            last_start: None,
        }
    }
}

impl CountdownState {
    /// Increases the target time by one minute (if idle or finished).
    pub fn increment_minutes(&mut self, minutes: u64) {
        if matches!(self.status, CountdownStatus::Idle | CountdownStatus::Finished) {
            self.target += Duration::from_secs(minutes * 60);
            self.remaining = self.target;
            self.status = CountdownStatus::Idle;
        }
    }

    /// Increases the target time by one second (if idle or finished).
    pub fn increment_seconds(&mut self, seconds: u64) {
        if matches!(self.status, CountdownStatus::Idle | CountdownStatus::Finished) {
            self.target += Duration::from_secs(seconds);
            self.remaining = self.target;
            self.status = CountdownStatus::Idle;
        }
    }

    /// Starts the countdown (if idle or finished).
    pub fn start(&mut self) {
        if matches!(self.status, CountdownStatus::Idle | CountdownStatus::Finished) {
            if self.target > Duration::ZERO {
                self.remaining = self.target;
                self.last_start = Some(Instant::now());
                self.status = CountdownStatus::Running;
            }
        }
    }

    /// Toggles pause (if running or paused).
    pub fn toggle_pause(&mut self) {
        match self.status {
            CountdownStatus::Running => {
                self.remaining = self.current_remaining();
                self.last_start = None;
                self.status = CountdownStatus::Paused;
            }
            CountdownStatus::Paused => {
                self.last_start = Some(Instant::now());
                self.status = CountdownStatus::Running;
            }
            _ => {}
        }
    }

    /// Resets to target (if running/paused) or clears target (if idle).
    pub fn reset(&mut self) {
        if matches!(self.status, CountdownStatus::Running | CountdownStatus::Paused) {
            self.remaining = self.target;
            self.last_start = None;
            self.status = CountdownStatus::Idle;
        } else if matches!(self.status, CountdownStatus::Idle | CountdownStatus::Finished) {
            self.target = Duration::ZERO;
            self.remaining = Duration::ZERO;
            self.last_start = None;
            self.status = CountdownStatus::Idle;
        }
    }

    /// Returns the current remaining time, accounting for the live running segment.
    pub fn current_remaining(&self) -> Duration {
        match self.status {
            CountdownStatus::Running => {
                let elapsed_since_start = self.last_start.map(|s| s.elapsed()).unwrap_or_default();
                self.target.saturating_sub(elapsed_since_start)
            }
            _ => self.remaining,
        }
    }

    /// Ticks the countdown: updates remaining time and detects completion.
    /// Returns `true` if the countdown just transitioned to `Finished`.
    pub fn tick(&mut self) -> bool {
        if self.status == CountdownStatus::Running {
            self.remaining = self.current_remaining();
            if self.remaining == Duration::ZERO {
                self.status = CountdownStatus::Finished;
                self.last_start = None;
                return true;
            }
        }
        false
    }
}

/// Formats a duration as `MM:SS` or `HH:MM:SS` after 60 minutes.
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_countdown_default_is_idle() {
        let state = CountdownState::default();
        assert_eq!(state.status, CountdownStatus::Idle);
        assert_eq!(state.target, Duration::ZERO);
        assert_eq!(state.remaining, Duration::ZERO);
    }

    #[test]
    fn test_countdown_increment_minutes() {
        let mut state = CountdownState::default();
        state.increment_minutes(5);
        assert_eq!(state.target, Duration::from_secs(300));
        assert_eq!(state.remaining, Duration::from_secs(300));
        assert_eq!(state.status, CountdownStatus::Idle);
    }

    #[test]
    fn test_countdown_increment_seconds() {
        let mut state = CountdownState::default();
        state.increment_minutes(1);
        state.increment_seconds(30);
        assert_eq!(state.target, Duration::from_secs(90));
        assert_eq!(state.remaining, Duration::from_secs(90));
    }

    #[test]
    fn test_countdown_increment_while_running_is_noop() {
        let mut state = CountdownState {
            status: CountdownStatus::Running,
            target: Duration::from_secs(60),
            remaining: Duration::from_secs(60),
            last_start: Some(Instant::now()),
        };
        state.increment_minutes(1);
        assert_eq!(state.target, Duration::from_secs(60));
    }

    #[test]
    fn test_countdown_start_from_idle() {
        let mut state = CountdownState::default();
        state.increment_minutes(1);
        state.start();
        assert_eq!(state.status, CountdownStatus::Running);
        assert!(state.last_start.is_some());
    }

    #[test]
    fn test_countdown_start_with_zero_target_is_noop() {
        let mut state = CountdownState::default();
        state.start();
        assert_eq!(state.status, CountdownStatus::Idle);
    }

    #[test]
    fn test_countdown_toggle_pause_from_running() {
        let mut state = CountdownState {
            status: CountdownStatus::Running,
            target: Duration::from_secs(60),
            remaining: Duration::from_secs(60),
            last_start: Some(Instant::now()),
        };
        state.toggle_pause();
        assert_eq!(state.status, CountdownStatus::Paused);
        assert!(state.last_start.is_none());
    }

    #[test]
    fn test_countdown_toggle_pause_from_paused() {
        let mut state = CountdownState {
            status: CountdownStatus::Paused,
            target: Duration::from_secs(60),
            remaining: Duration::from_secs(30),
            last_start: None,
        };
        state.toggle_pause();
        assert_eq!(state.status, CountdownStatus::Running);
        assert!(state.last_start.is_some());
    }

    #[test]
    fn test_countdown_reset_while_running() {
        let mut state = CountdownState {
            status: CountdownStatus::Running,
            target: Duration::from_secs(60),
            remaining: Duration::from_secs(30),
            last_start: Some(Instant::now()),
        };
        state.reset();
        assert_eq!(state.status, CountdownStatus::Idle);
        assert_eq!(state.remaining, Duration::from_secs(60));
        assert!(state.last_start.is_none());
    }

    #[test]
    fn test_countdown_reset_while_idle_clears_target() {
        let mut state = CountdownState {
            status: CountdownStatus::Idle,
            target: Duration::from_secs(60),
            remaining: Duration::from_secs(60),
            last_start: None,
        };
        state.reset();
        assert_eq!(state.status, CountdownStatus::Idle);
        assert_eq!(state.target, Duration::ZERO);
        assert_eq!(state.remaining, Duration::ZERO);
    }

    #[test]
    fn test_countdown_tick_detects_completion() {
        let mut state = CountdownState {
            status: CountdownStatus::Running,
            target: Duration::from_millis(100),
            remaining: Duration::from_millis(100),
            last_start: Some(Instant::now()),
        };
        std::thread::sleep(Duration::from_millis(150));
        let finished = state.tick();
        assert!(finished);
        assert_eq!(state.status, CountdownStatus::Finished);
    }

    #[test]
    fn test_countdown_tick_while_idle_returns_false() {
        let mut state = CountdownState::default();
        let finished = state.tick();
        assert!(!finished);
        assert_eq!(state.status, CountdownStatus::Idle);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00");
        assert_eq!(format_duration(Duration::from_secs(300)), "05:00");
        assert_eq!(format_duration(Duration::from_secs(3661)), "01:01:01");
    }
}
