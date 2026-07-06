use crate::PowerAction;

/// Information about a currently scheduled power action.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScheduledActionInfo {
    /// The power action that is scheduled.
    pub action: PowerAction,
    /// Remaining time in seconds until the action executes.
    pub remaining_seconds: u64,
    /// Total originally scheduled delay in seconds.
    pub total_delay_seconds: u64,
}
