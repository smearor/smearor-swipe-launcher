/// A tracked process with its termination policy.
#[derive(Clone, Debug)]
pub struct TrackedProcess {
    /// The PID of the running process.
    pub pid: u32,
    /// Whether to terminate the process when the launcher exits.
    pub terminate_on_exit: bool,
}
