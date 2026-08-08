/// Device rendering parameters extracted from `MacroPadDeviceMetadata`.
///
/// Contains the fields needed by the rendering pipeline to render and
/// dispatch button images to a MacroPad device.
#[derive(Debug, Clone)]
pub struct DeviceRenderInfo {
    /// Unique device identifier (serial number or composite ID).
    pub device_id: String,
    /// Driver/service that manages this device (e.g. "streamdeck", "loupedeck").
    pub driver: String,
    /// Number of keys on the device.
    pub key_count: u8,
    /// Number of columns in the device's button grid.
    pub key_columns: u8,
    /// Key resolution width in pixels.
    pub key_width: u32,
    /// Key resolution height in pixels.
    pub key_height: u32,
}
