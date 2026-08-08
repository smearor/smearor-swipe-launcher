use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Arguments for the `streamdeck_set_brightness` and `loupedeck_set_brightness` MCP tools.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct MacroPadSetBrightnessArgs {
    /// Brightness percentage (0-100)
    pub brightness: u8,
    /// Device serial number (empty = all connected devices)
    pub device_id: Option<String>,
}
