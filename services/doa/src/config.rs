use serde::Deserialize;

/// Configuration for the DoA service.
#[derive(Debug, Clone, Deserialize)]
pub struct DoaServiceConfig {
    /// Polling interval for DoA reads in milliseconds.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    /// Whether to enable MCP tool registration for this service.
    #[serde(default = "default_mcp_enabled")]
    pub mcp_enabled: bool,
    /// Optional product ID filter. If set, only devices with this PID are matched.
    #[serde(default)]
    pub product_id: Option<u16>,
    /// Reconnection delay in milliseconds when the USB device is lost.
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_ms: u64,
    /// Rotation offset in degrees (-360 to 360) to calibrate the DoA angle to the
    /// physical table orientation. The raw DSP angle is rotated by this offset
    /// before mapping to a compass direction. Positive values rotate clockwise,
    /// negative values counter-clockwise. Values outside ±360 are wrapped via
    /// `rem_euclid(360)`. Use this when the microphone array's 0° axis does not
    /// align with the table's North/reference direction.
    /// Example: if the DSP 0° points 90° clockwise from table North, set offset = -90.
    #[serde(default = "default_rotation_offset")]
    pub rotation_offset: i16,
}

impl Default for DoaServiceConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: default_poll_interval(),
            mcp_enabled: default_mcp_enabled(),
            product_id: None,
            reconnect_delay_ms: default_reconnect_delay(),
            rotation_offset: default_rotation_offset(),
        }
    }
}

fn default_poll_interval() -> u64 {
    150
}

fn default_mcp_enabled() -> bool {
    true
}

fn default_reconnect_delay() -> u64 {
    2000
}

fn default_rotation_offset() -> i16 {
    0
}

#[cfg(test)]
mod tests {
    use super::DoaServiceConfig;

    #[test]
    fn test_default_values() {
        let config = DoaServiceConfig::default();
        assert_eq!(config.poll_interval_ms, 150);
        assert!(config.mcp_enabled);
        assert_eq!(config.product_id, None);
        assert_eq!(config.reconnect_delay_ms, 2000);
        assert_eq!(config.rotation_offset, 0);
    }

    #[test]
    fn test_parse_empty_json_uses_defaults() {
        let json = serde_json::json!({});
        let config: DoaServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.poll_interval_ms, 150);
        assert!(config.mcp_enabled);
        assert_eq!(config.product_id, None);
        assert_eq!(config.reconnect_delay_ms, 2000);
        assert_eq!(config.rotation_offset, 0);
    }

    #[test]
    fn test_parse_partial_json() {
        let json = serde_json::json!({
            "poll_interval_ms": 200,
            "rotation_offset": -90
        });
        let config: DoaServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.poll_interval_ms, 200);
        assert!(config.mcp_enabled);
        assert_eq!(config.product_id, None);
        assert_eq!(config.reconnect_delay_ms, 2000);
        assert_eq!(config.rotation_offset, -90);
    }

    #[test]
    fn test_parse_full_json() {
        let json = serde_json::json!({
            "poll_interval_ms": 50,
            "mcp_enabled": false,
            "product_id": 0x0021,
            "reconnect_delay_ms": 5000,
            "rotation_offset": 270
        });
        let config: DoaServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.poll_interval_ms, 50);
        assert!(!config.mcp_enabled);
        assert_eq!(config.product_id, Some(0x0021));
        assert_eq!(config.reconnect_delay_ms, 5000);
        assert_eq!(config.rotation_offset, 270);
    }

    #[test]
    fn test_parse_product_id_none() {
        let json = serde_json::json!({
            "product_id": null
        });
        let config: DoaServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.product_id, None);
    }

    #[test]
    fn test_parse_negative_rotation_offset() {
        let json = serde_json::json!({
            "rotation_offset": -180
        });
        let config: DoaServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.rotation_offset, -180);
    }
}
