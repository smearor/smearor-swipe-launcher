use crate::countdown_state::CountdownState;
use crate::timer_state::TimerState;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::RwLock;

/// Shared state for a span group, holding either Timer or Countdown state.
#[derive(Debug, Default)]
pub enum SpanGroupState {
    /// Timer (stopwatch) state.
    #[default]
    None,
    /// Timer (stopwatch) state.
    Timer(TimerState),
    /// Countdown timer state.
    Countdown(CountdownState),
}

impl SpanGroupState {
    /// Returns a mutable reference to the inner `TimerState`, if this is a Timer variant.
    pub fn as_timer(&mut self) -> Option<&mut TimerState> {
        match self {
            SpanGroupState::Timer(state) => Some(state),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner `CountdownState`, if this is a Countdown variant.
    pub fn as_countdown(&mut self) -> Option<&mut CountdownState> {
        match self {
            SpanGroupState::Countdown(state) => Some(state),
            _ => None,
        }
    }

    /// Returns a shared reference to the inner `TimerState`, if this is a Timer variant.
    pub fn as_timer_ref(&self) -> Option<&TimerState> {
        match self {
            SpanGroupState::Timer(state) => Some(state),
            _ => None,
        }
    }

    /// Returns a shared reference to the inner `CountdownState`, if this is a Countdown variant.
    pub fn as_countdown_ref(&self) -> Option<&CountdownState> {
        match self {
            SpanGroupState::Countdown(state) => Some(state),
            _ => None,
        }
    }
}

/// Registry of shared span group states, keyed by span_group name.
///
/// Because all instances of the same `.so` share the same static, this
/// allows multiple widget instances in the same span group to access
/// the same `Arc<Mutex<SpanGroupState>>`.
static SPAN_STATE_REGISTRY: LazyLock<RwLock<HashMap<String, Arc<Mutex<SpanGroupState>>>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

/// Looks up or creates shared state for the given span group.
///
/// If `span_group` is `None`, a private (unregistered) state is created.
pub fn lookup_or_create_state(span_group: Option<&str>, initial: SpanGroupState) -> Arc<Mutex<SpanGroupState>> {
    if let Some(group) = span_group {
        let mut registry = SPAN_STATE_REGISTRY.write().unwrap();
        registry.entry(group.to_string()).or_insert_with(|| Arc::new(Mutex::new(initial))).clone()
    } else {
        Arc::new(Mutex::new(initial))
    }
}

/// Cleans up the registry entry for a span group if no more instances reference it.
pub fn cleanup_state(span_group: Option<&str>, _state: &Arc<Mutex<SpanGroupState>>) {
    if let Some(group) = span_group {
        if let Ok(mut registry) = SPAN_STATE_REGISTRY.write() {
            if let Some(arc) = registry.get(group) {
                if Arc::strong_count(arc) <= 1 {
                    registry.remove(group);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_or_create_state_with_group() {
        let state1 = lookup_or_create_state(Some("test_group"), SpanGroupState::default());
        let state2 = lookup_or_create_state(Some("test_group"), SpanGroupState::default());
        assert!(Arc::ptr_eq(&state1, &state2));
        cleanup_state(Some("test_group"), &state1);
        cleanup_state(Some("test_group"), &state2);
    }

    #[test]
    fn test_lookup_or_create_state_without_group() {
        let state1 = lookup_or_create_state(None, SpanGroupState::default());
        let state2 = lookup_or_create_state(None, SpanGroupState::default());
        assert!(!Arc::ptr_eq(&state1, &state2));
    }

    #[test]
    fn test_different_groups_are_independent() {
        let state1 = lookup_or_create_state(Some("group_a"), SpanGroupState::default());
        let state2 = lookup_or_create_state(Some("group_b"), SpanGroupState::default());
        assert!(!Arc::ptr_eq(&state1, &state2));
        cleanup_state(Some("group_a"), &state1);
        cleanup_state(Some("group_b"), &state2);
    }
}
