use serde::Deserialize;
use serde::Serialize;

/// Views available in the DoA widget for tile rotation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum DoaView {
    /// Compass view: shows the current angle as a compass needle.
    #[default]
    Compass,
    /// Direction view: shows the mapped table side (N/E/S/W) as text + icon.
    Direction,
    /// Device info view: shows connection status, vendor/product ID, speech activity.
    DeviceInfo,
}
