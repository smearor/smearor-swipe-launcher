/// Connection state of a network device as reported by NetworkManager.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NetworkConnectionState {
    /// Device is disconnected.
    #[default]
    Disconnected,
    /// Device is connecting.
    Connecting,
    /// Device is fully connected.
    Connected,
    /// Device is in a failed state.
    Failed,
    /// Device is unavailable.
    Unavailable,
}
