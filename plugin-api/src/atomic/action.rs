//! Atomic-specific action types for atomic widget MacroPad input.

use std::str::FromStr;

/// MacroPad actions that an atomic widget can dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AtomicAction {
    /// Single-click action.
    Click,
    /// Long-press action.
    Longpress,
    /// Hold start (push-to-talk begin).
    HoldStart,
    /// Hold stop (push-to-talk end).
    HoldStop,
    /// Double press action.
    DoublePress,
    /// Compound longpress action.
    CompoundLongpress,
}

/// Error returned when parsing an unknown atomic action string.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("unknown atomic action")]
pub struct UnknownAtomicActionError;

impl FromStr for AtomicAction {
    type Err = UnknownAtomicActionError;

    /// Parses an atomic action from its string representation.
    ///
    /// Returns `Err` if the string does not match a known action.
    fn from_str(action: &str) -> Result<Self, Self::Err> {
        match action {
            "click" => Ok(Self::Click),
            "longpress" => Ok(Self::Longpress),
            "hold_start" => Ok(Self::HoldStart),
            "hold_stop" => Ok(Self::HoldStop),
            "double_press" => Ok(Self::DoublePress),
            "compound_longpress" => Ok(Self::CompoundLongpress),
            _ => Err(UnknownAtomicActionError),
        }
    }
}

impl AsRef<str> for AtomicAction {
    /// Returns the string representation of this action.
    fn as_ref(&self) -> &str {
        match self {
            Self::Click => "click",
            Self::Longpress => "longpress",
            Self::HoldStart => "hold_start",
            Self::HoldStop => "hold_stop",
            Self::DoublePress => "double_press",
            Self::CompoundLongpress => "compound_longpress",
        }
    }
}

/// Hook for widgets that need to react to MacroPad actions with internal logic.
///
/// Implemented by widgets that manage internal state (e.g. Timer, Countdown).
/// Called by the atomic widget macro's `dispatch_action` method, giving the
/// widget an opportunity to update its state before the config-driven
/// topic/payload dispatch runs.
///
/// The `span_index` parameter identifies which button in the span group
/// was pressed, allowing per-button action differentiation.
pub trait SpanActionHandler {
    /// Called when a MacroPad action is dispatched to this widget instance.
    ///
    /// `action` is the trigger type (click, longpress, etc.).
    /// `span_index` is this instance's index within its span group (0 if not in a group).
    fn on_span_action(&self, action: AtomicAction, span_index: u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_action_from_str() {
        assert_eq!(AtomicAction::from_str("click"), Ok(AtomicAction::Click));
        assert_eq!(AtomicAction::from_str("longpress"), Ok(AtomicAction::Longpress));
        assert_eq!(AtomicAction::from_str("hold_start"), Ok(AtomicAction::HoldStart));
        assert_eq!(AtomicAction::from_str("hold_stop"), Ok(AtomicAction::HoldStop));
        assert_eq!(AtomicAction::from_str("double_press"), Ok(AtomicAction::DoublePress));
        assert_eq!(AtomicAction::from_str("compound_longpress"), Ok(AtomicAction::CompoundLongpress));
        assert!(AtomicAction::from_str("unknown").is_err());
    }

    #[test]
    fn test_atomic_action_as_ref() {
        assert_eq!(AtomicAction::Click.as_ref(), "click");
        assert_eq!(AtomicAction::Longpress.as_ref(), "longpress");
        assert_eq!(AtomicAction::HoldStart.as_ref(), "hold_start");
        assert_eq!(AtomicAction::HoldStop.as_ref(), "hold_stop");
        assert_eq!(AtomicAction::DoublePress.as_ref(), "double_press");
        assert_eq!(AtomicAction::CompoundLongpress.as_ref(), "compound_longpress");
    }
}
