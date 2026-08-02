/// Dimming phases for the auto-dimming state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimmingPhase {
    /// Device is active, full brightness.
    Active,
    /// Fading from active to dimmed brightness.
    FadingDown,
    /// Device is dimmed, waiting for activity.
    Dimmed,
    /// Fading from dimmed to active brightness.
    FadingUp,
}
