use smearor_model_instance_control::LauncherInstanceLifecycle;
use smearor_model_instance_control::LauncherInstanceLifecycleTransitionError;
use std::sync::Mutex;
use tracing::debug;
use tracing::error;

/// RAII guard for lifecycle state transitions.
///
/// When created, it transitions the lifecycle state to the `intermediate` state.
/// On `Drop`, if `complete()` was not called, it rolls back to the `rollback` state.
/// If `complete()` was called, it transitions to the `target` state.
///
/// This ensures that if a lifecycle method fails or panics, the lifecycle state
/// is automatically rolled back to a known-good state.
pub struct LifecycleGuard<'a> {
    lifecycle: &'a Mutex<LauncherInstanceLifecycle>,
    rollback: LauncherInstanceLifecycle,
    target: LauncherInstanceLifecycle,
    completed: bool,
}

impl<'a> LifecycleGuard<'a> {
    /// Create a new `LifecycleGuard`.
    ///
    /// Transitions the lifecycle state from `expected_current` to `intermediate`.
    /// On drop, rolls back to `rollback` if `complete()` was not called,
    /// or transitions to `target` if `complete()` was called.
    pub fn new(
        lifecycle: &'a Mutex<LauncherInstanceLifecycle>,
        expected_current: LauncherInstanceLifecycle,
        intermediate: LauncherInstanceLifecycle,
        rollback: LauncherInstanceLifecycle,
        target: LauncherInstanceLifecycle,
    ) -> Result<Self, LauncherInstanceLifecycleTransitionError> {
        let mut guard = lifecycle.lock().map_err(|_| LauncherInstanceLifecycleTransitionError::UnexpectedState {
            current: LauncherInstanceLifecycle::Loading,
            expected: expected_current.clone(),
        })?;

        if *guard != expected_current {
            return Err(LauncherInstanceLifecycleTransitionError::UnexpectedState {
                current: guard.clone(),
                expected: expected_current.clone(),
            });
        }

        expected_current.validate_transition_to(intermediate.clone())?;
        *guard = intermediate;

        Ok(LifecycleGuard {
            lifecycle,
            rollback,
            target,
            completed: false,
        })
    }

    /// Mark the transition as completed.
    ///
    /// On drop, the lifecycle state will be transitioned to `target` instead of `rollback`.
    pub fn complete(&mut self) {
        self.completed = true;
    }
}

impl Drop for LifecycleGuard<'_> {
    fn drop(&mut self) {
        let final_state = if self.completed { self.target.clone() } else { self.rollback.clone() };

        match self.lifecycle.lock() {
            Ok(mut guard) => {
                if *guard != final_state {
                    debug!("LifecycleGuard dropping: transitioning to {:?}", final_state);
                    *guard = final_state;
                }
            }
            Err(e) => {
                error!("LifecycleGuard drop: failed to lock lifecycle mutex: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_completes_to_target() {
        let lifecycle = Mutex::new(LauncherInstanceLifecycle::Ready);
        let mut guard = LifecycleGuard::new(
            &lifecycle,
            LauncherInstanceLifecycle::Ready,
            LauncherInstanceLifecycle::Starting,
            LauncherInstanceLifecycle::Ready,
            LauncherInstanceLifecycle::Running,
        )
        .unwrap();
        assert_eq!(*lifecycle.lock().unwrap(), LauncherInstanceLifecycle::Starting);
        guard.complete();
        drop(guard);
        assert_eq!(*lifecycle.lock().unwrap(), LauncherInstanceLifecycle::Running);
    }

    #[test]
    fn test_guard_rolls_back_on_drop_without_complete() {
        let lifecycle = Mutex::new(LauncherInstanceLifecycle::Ready);
        {
            let _guard = LifecycleGuard::new(
                &lifecycle,
                LauncherInstanceLifecycle::Ready,
                LauncherInstanceLifecycle::Starting,
                LauncherInstanceLifecycle::Ready,
                LauncherInstanceLifecycle::Running,
            )
            .unwrap();
            assert_eq!(*lifecycle.lock().unwrap(), LauncherInstanceLifecycle::Starting);
        }
        assert_eq!(*lifecycle.lock().unwrap(), LauncherInstanceLifecycle::Ready);
    }

    #[test]
    fn test_guard_rejects_wrong_expected_state() {
        let lifecycle = Mutex::new(LauncherInstanceLifecycle::Running);
        let result = LifecycleGuard::new(
            &lifecycle,
            LauncherInstanceLifecycle::Ready,
            LauncherInstanceLifecycle::Starting,
            LauncherInstanceLifecycle::Ready,
            LauncherInstanceLifecycle::Running,
        );
        assert!(result.is_err());
        assert_eq!(*lifecycle.lock().unwrap(), LauncherInstanceLifecycle::Running);
    }

    #[test]
    fn test_guard_rejects_invalid_transition() {
        let lifecycle = Mutex::new(LauncherInstanceLifecycle::Ready);
        let result = LifecycleGuard::new(
            &lifecycle,
            LauncherInstanceLifecycle::Ready,
            LauncherInstanceLifecycle::Running,
            LauncherInstanceLifecycle::Ready,
            LauncherInstanceLifecycle::Running,
        );
        assert!(result.is_err());
        assert_eq!(*lifecycle.lock().unwrap(), LauncherInstanceLifecycle::Ready);
    }
}
