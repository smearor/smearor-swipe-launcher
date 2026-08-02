/// Device-specific metadata for a MacroPad instance.
///
/// Attached to a `LauncherInstance` when a MacroPad device is connected.
/// None for GTK instances.
#[derive(Clone, Debug)]
pub struct MacroPadDeviceMetadata {
    /// Unique device identifier (serial number or composite ID).
    pub device_id: String,
    /// Number of keys on the device.
    pub key_count: u8,
    /// Number of columns in the device's button grid.
    ///
    /// Used by the host to map 2D span group positions to physical button
    /// indices. For devices with a single row, this equals `key_count`.
    pub key_columns: u8,
    /// Key resolution width in pixels.
    pub key_width: u32,
    /// Key resolution height in pixels.
    pub key_height: u32,
    /// Driver/service that manages this device (e.g. "streamdeck", "loupedeck").
    pub driver: String,
}
