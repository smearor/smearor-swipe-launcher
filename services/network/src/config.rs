use serde::Deserialize;
use typed_builder::TypedBuilder;

/// Configuration for the network service.
#[derive(Debug, Clone, Deserialize, TypedBuilder)]
#[serde(default)]
pub struct NetworkServiceConfig {
    /// Interval in seconds for refreshing network status.
    #[builder(default = 5)]
    pub refresh_interval_seconds: u64,
    /// Whether to enable WLAN scanning.
    #[builder(default = true)]
    pub enable_wifi_scan: bool,
    /// Whether to enable VPN profile management.
    #[builder(default = true)]
    pub enable_vpn_management: bool,
    /// Whether to enable throughput monitoring.
    #[builder(default = true)]
    pub enable_throughput_monitoring: bool,
    /// Interval in seconds for throughput sampling.
    #[builder(default = 2)]
    pub throughput_sample_interval_seconds: u64,
    /// URL for public IP lookup (via internal HTTP service).
    #[builder(default = String::from("https://api.ipify.org?format=json"))]
    pub public_ip_url: String,
}

impl Default for NetworkServiceConfig {
    fn default() -> Self {
        Self {
            refresh_interval_seconds: 5,
            enable_wifi_scan: true,
            enable_vpn_management: true,
            enable_throughput_monitoring: true,
            throughput_sample_interval_seconds: 2,
            public_ip_url: String::from("https://api.ipify.org?format=json"),
        }
    }
}
