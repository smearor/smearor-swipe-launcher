use serde::Deserialize;
use serde::Serialize;

/// Lifecycle states for launcher instances.
///
/// Each instance transitions through these states during its lifetime.
/// Stable states are `Ready` and `Running`. Intermediate states (`Loading`,
/// `Starting`, `Stopping`, `Unloading`) are transient — they exist only during
/// the execution of a lifecycle method and roll back on failure.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LauncherInstanceLifecycle {
    /// Instance is being loaded (parsing config, loading plugins).
    Loading,
    /// Instance is loaded but not started. Plugins are loaded, no window/areas.
    #[default]
    Ready,
    /// Instance is transitioning from `Ready` to `Running` (building window/areas).
    Starting,
    /// Instance is running with its window or headless areas active.
    Running,
    /// Instance is transitioning from `Running` to `Ready` (closing window/areas).
    Stopping,
    /// Instance is being fully removed (unloading plugins, removing watchers).
    Unloading,
}

/// Error returned when a lifecycle transition is not allowed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LauncherInstanceLifecycleTransitionError {
    /// The transition from `current` to `target` is not a valid state transition.
    #[error("Invalid lifecycle transition: {current:?} → {target:?}")]
    InvalidTransition {
        /// The current lifecycle state.
        current: LauncherInstanceLifecycle,
        /// The target lifecycle state.
        target: LauncherInstanceLifecycle,
    },
    /// The instance is not in the expected state for this operation.
    #[error("Instance is in state {current:?}, expected {expected:?}")]
    UnexpectedState {
        /// The current lifecycle state.
        current: LauncherInstanceLifecycle,
        /// The expected lifecycle state.
        expected: LauncherInstanceLifecycle,
    },
}

impl LauncherInstanceLifecycle {
    /// Returns the lowercase string representation used in persistence and JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            LauncherInstanceLifecycle::Loading => "loading",
            LauncherInstanceLifecycle::Ready => "ready",
            LauncherInstanceLifecycle::Starting => "starting",
            LauncherInstanceLifecycle::Running => "running",
            LauncherInstanceLifecycle::Stopping => "stopping",
            LauncherInstanceLifecycle::Unloading => "unloading",
        }
    }

    /// Parse from a lowercase string representation.
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "loading" => Ok(LauncherInstanceLifecycle::Loading),
            "ready" => Ok(LauncherInstanceLifecycle::Ready),
            "starting" => Ok(LauncherInstanceLifecycle::Starting),
            "running" => Ok(LauncherInstanceLifecycle::Running),
            "stopping" => Ok(LauncherInstanceLifecycle::Stopping),
            "unloading" => Ok(LauncherInstanceLifecycle::Unloading),
            other => Err(format!("unknown lifecycle state: {}", other)),
        }
    }

    /// Validates whether transitioning from `self` to `target` is allowed.
    ///
    /// Valid transitions:
    /// - `Loading → Ready`
    /// - `Ready → Starting`
    /// - `Ready → Unloading`
    /// - `Starting → Running`
    /// - `Running → Stopping`
    /// - `Stopping → Ready`
    /// - `Unloading → Ready` (rollback on failure)
    pub fn validate_transition_to(&self, target: LauncherInstanceLifecycle) -> Result<(), LauncherInstanceLifecycleTransitionError> {
        let valid = matches!(
            (self, &target),
            (LauncherInstanceLifecycle::Loading, LauncherInstanceLifecycle::Ready)
                | (LauncherInstanceLifecycle::Ready, LauncherInstanceLifecycle::Starting)
                | (LauncherInstanceLifecycle::Ready, LauncherInstanceLifecycle::Unloading)
                | (LauncherInstanceLifecycle::Starting, LauncherInstanceLifecycle::Running)
                | (LauncherInstanceLifecycle::Running, LauncherInstanceLifecycle::Stopping)
                | (LauncherInstanceLifecycle::Stopping, LauncherInstanceLifecycle::Ready)
                | (LauncherInstanceLifecycle::Unloading, LauncherInstanceLifecycle::Ready)
        );
        if valid {
            Ok(())
        } else {
            Err(LauncherInstanceLifecycleTransitionError::InvalidTransition { current: self.clone(), target })
        }
    }

    /// Validates whether transitioning from `previous` to `self` was a valid incoming transition.
    ///
    /// This is the inverse of `validate_transition_to`, used for rollback validation.
    pub fn validate_transition_from(&self, previous: LauncherInstanceLifecycle) -> Result<(), LauncherInstanceLifecycleTransitionError> {
        previous.validate_transition_to(self.clone())
    }

    /// Returns `true` if this state is a stable state (not an intermediate transition state).
    pub fn is_stable(&self) -> bool {
        matches!(self, LauncherInstanceLifecycle::Ready | LauncherInstanceLifecycle::Running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(
            LauncherInstanceLifecycle::Loading
                .validate_transition_to(LauncherInstanceLifecycle::Ready)
                .is_ok()
        );
        assert!(
            LauncherInstanceLifecycle::Ready
                .validate_transition_to(LauncherInstanceLifecycle::Starting)
                .is_ok()
        );
        assert!(
            LauncherInstanceLifecycle::Ready
                .validate_transition_to(LauncherInstanceLifecycle::Unloading)
                .is_ok()
        );
        assert!(
            LauncherInstanceLifecycle::Starting
                .validate_transition_to(LauncherInstanceLifecycle::Running)
                .is_ok()
        );
        assert!(
            LauncherInstanceLifecycle::Running
                .validate_transition_to(LauncherInstanceLifecycle::Stopping)
                .is_ok()
        );
        assert!(
            LauncherInstanceLifecycle::Stopping
                .validate_transition_to(LauncherInstanceLifecycle::Ready)
                .is_ok()
        );
        assert!(
            LauncherInstanceLifecycle::Unloading
                .validate_transition_to(LauncherInstanceLifecycle::Ready)
                .is_ok()
        );
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(
            LauncherInstanceLifecycle::Loading
                .validate_transition_to(LauncherInstanceLifecycle::Running)
                .is_err()
        );
        assert!(
            LauncherInstanceLifecycle::Ready
                .validate_transition_to(LauncherInstanceLifecycle::Running)
                .is_err()
        );
        assert!(
            LauncherInstanceLifecycle::Running
                .validate_transition_to(LauncherInstanceLifecycle::Ready)
                .is_err()
        );
        assert!(
            LauncherInstanceLifecycle::Starting
                .validate_transition_to(LauncherInstanceLifecycle::Ready)
                .is_err()
        );
        assert!(
            LauncherInstanceLifecycle::Stopping
                .validate_transition_to(LauncherInstanceLifecycle::Running)
                .is_err()
        );
    }

    #[test]
    fn test_as_str_roundtrip() {
        for state in [
            LauncherInstanceLifecycle::Loading,
            LauncherInstanceLifecycle::Ready,
            LauncherInstanceLifecycle::Starting,
            LauncherInstanceLifecycle::Running,
            LauncherInstanceLifecycle::Stopping,
            LauncherInstanceLifecycle::Unloading,
        ] {
            let s = state.as_str();
            let parsed = LauncherInstanceLifecycle::from_str(s).unwrap();
            assert_eq!(state, parsed, "roundtrip failed for {}", s);
        }
    }

    #[test]
    fn test_from_str_unknown() {
        assert!(LauncherInstanceLifecycle::from_str("unknown").is_err());
    }

    #[test]
    fn test_is_stable() {
        assert!(LauncherInstanceLifecycle::Ready.is_stable());
        assert!(LauncherInstanceLifecycle::Running.is_stable());
        assert!(!LauncherInstanceLifecycle::Loading.is_stable());
        assert!(!LauncherInstanceLifecycle::Starting.is_stable());
        assert!(!LauncherInstanceLifecycle::Stopping.is_stable());
        assert!(!LauncherInstanceLifecycle::Unloading.is_stable());
    }

    #[test]
    fn test_default_is_ready() {
        assert_eq!(LauncherInstanceLifecycle::default(), LauncherInstanceLifecycle::Ready);
    }

    #[test]
    fn test_validate_transition_from() {
        assert!(
            LauncherInstanceLifecycle::Running
                .validate_transition_from(LauncherInstanceLifecycle::Starting)
                .is_ok()
        );
        assert!(
            LauncherInstanceLifecycle::Ready
                .validate_transition_from(LauncherInstanceLifecycle::Loading)
                .is_ok()
        );
        assert!(
            LauncherInstanceLifecycle::Ready
                .validate_transition_from(LauncherInstanceLifecycle::Running)
                .is_err()
        );
    }
}
