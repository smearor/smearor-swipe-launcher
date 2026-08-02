use std::time::Duration;
use std::time::Instant;

/// Whether the timer is running, paused, or idle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimerStatus {
    /// Timer has not been started yet.
    #[default]
    Idle,
    /// Timer is actively counting up.
    Running,
    /// Timer was running and is now paused.
    Paused,
}

/// State of the Timer (stopwatch) widget.
#[derive(Clone, Debug, Default)]
pub struct TimerState {
    /// Whether the timer is running, paused, or idle.
    pub status: TimerStatus,
    /// Elapsed time accumulated while running.
    pub elapsed: Duration,
    /// Instant when the current run segment started (for computing live elapsed).
    pub last_start: Option<Instant>,
}

impl TimerState {
    /// Starts or resumes the timer.
    pub fn start(&mut self) {
        match self.status {
            TimerStatus::Idle | TimerStatus::Paused => {
                self.last_start = Some(Instant::now());
                self.status = TimerStatus::Running;
            }
            TimerStatus::Running => {}
        }
    }

    /// Pauses the timer if it is running.
    pub fn pause(&mut self) {
        if self.status == TimerStatus::Running {
            self.elapsed = self.current_elapsed();
            self.last_start = None;
            self.status = TimerStatus::Paused;
        }
    }

    /// Resets the timer to idle and zero elapsed.
    pub fn reset(&mut self) {
        self.status = TimerStatus::Idle;
        self.elapsed = Duration::ZERO;
        self.last_start = None;
    }

    /// Returns the current elapsed time, including the live running segment.
    pub fn current_elapsed(&self) -> Duration {
        match self.status {
            TimerStatus::Running => {
                let live = self.last_start.map(|s| s.elapsed()).unwrap_or_default();
                self.elapsed + live
            }
            _ => self.elapsed,
        }
    }
}

/// Formats a duration as `MM:SS` or `HH:MM:SS` after 60 minutes.
pub fn format_elapsed(duration: Duration) -> String {
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
    fn test_timer_default_is_idle() {
        let state = TimerState::default();
        assert_eq!(state.status, TimerStatus::Idle);
        assert_eq!(state.elapsed, Duration::ZERO);
    }

    #[test]
    fn test_timer_start_from_idle() {
        let mut state = TimerState::default();
        state.start();
        assert_eq!(state.status, TimerStatus::Running);
        assert!(state.last_start.is_some());
    }

    #[test]
    fn test_timer_start_from_paused() {
        let mut state = TimerState {
            status: TimerStatus::Paused,
            elapsed: Duration::from_secs(10),
            last_start: None,
        };
        state.start();
        assert_eq!(state.status, TimerStatus::Running);
    }

    #[test]
    fn test_timer_start_while_running_is_noop() {
        let mut state = TimerState {
            status: TimerStatus::Running,
            elapsed: Duration::from_secs(5),
            last_start: Some(Instant::now()),
        };
        let start_before = state.last_start;
        state.start();
        assert_eq!(state.status, TimerStatus::Running);
        assert_eq!(state.last_start, start_before);
    }

    #[test]
    fn test_timer_pause_from_running() {
        let mut state = TimerState {
            status: TimerStatus::Running,
            elapsed: Duration::from_secs(5),
            last_start: Some(Instant::now()),
        };
        state.pause();
        assert_eq!(state.status, TimerStatus::Paused);
        assert!(state.last_start.is_none());
        assert!(state.elapsed >= Duration::from_secs(5));
    }

    #[test]
    fn test_timer_pause_from_idle_is_noop() {
        let mut state = TimerState::default();
        state.pause();
        assert_eq!(state.status, TimerStatus::Idle);
    }

    #[test]
    fn test_timer_reset() {
        let mut state = TimerState {
            status: TimerStatus::Running,
            elapsed: Duration::from_secs(30),
            last_start: Some(Instant::now()),
        };
        state.reset();
        assert_eq!(state.status, TimerStatus::Idle);
        assert_eq!(state.elapsed, Duration::ZERO);
        assert!(state.last_start.is_none());
    }

    #[test]
    fn test_format_elapsed_under_one_hour() {
        assert_eq!(format_elapsed(Duration::from_secs(0)), "00:00");
        assert_eq!(format_elapsed(Duration::from_secs(23)), "00:23");
        assert_eq!(format_elapsed(Duration::from_secs(65)), "01:05");
        assert_eq!(format_elapsed(Duration::from_secs(3599)), "59:59");
    }

    #[test]
    fn test_format_elapsed_over_one_hour() {
        assert_eq!(format_elapsed(Duration::from_secs(3600)), "01:00:00");
        assert_eq!(format_elapsed(Duration::from_secs(3661)), "01:01:01");
    }
}
