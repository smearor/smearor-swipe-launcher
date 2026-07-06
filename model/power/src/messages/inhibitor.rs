/// Information about a single inhibitor lock that blocks a power action.
#[stabby::stabby(no_opt)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InhibitorInfo {
    /// Process name that holds the inhibitor lock.
    pub process_name: stabby::string::String,
    /// Reason for the inhibitor lock.
    pub reason: stabby::string::String,
    /// What the inhibitor blocks (e.g., "shutdown", "sleep", "idle").
    pub what: stabby::string::String,
    /// Who registered the inhibitor (e.g., "APT", "Firefox").
    pub who: stabby::string::String,
}
