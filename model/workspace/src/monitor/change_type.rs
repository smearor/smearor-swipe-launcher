use serde::Deserialize;
use serde::Serialize;

/// Type of monitor change.
#[repr(u8)]
#[stabby::stabby]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonitorChangeType {
    /// Monitor was connected.
    #[default]
    Connected,
    /// Monitor was disconnected.
    Disconnected,
}
