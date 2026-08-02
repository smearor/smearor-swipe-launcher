use smearor_model_mcp::UnknownResourceError;
use std::fmt::Display;
use std::str::FromStr;

/// MCP resources exposed by the sysinfo service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysinfoMcpResources {
    /// CPU usage and temperature.
    Cpu,
    /// Per-component temperature readings.
    TemperatureComponents,
    /// Memory usage (total, used, available).
    Memory,
    /// Battery level and charging status.
    Battery,
    /// Disk mount points and I/O throughput.
    Disks,
    /// Network throughput (received/transmitted bytes per second).
    Network,
    /// System uptime and load averages.
    Uptime,
}

impl AsRef<str> for SysinfoMcpResources {
    fn as_ref(&self) -> &str {
        match self {
            Self::Cpu => "sysinfo://cpu",
            Self::TemperatureComponents => "sysinfo://temperature-components",
            Self::Memory => "sysinfo://memory",
            Self::Battery => "sysinfo://battery",
            Self::Disks => "sysinfo://disks",
            Self::Network => "sysinfo://network",
            Self::Uptime => "sysinfo://uptime",
        }
    }
}

impl FromStr for SysinfoMcpResources {
    type Err = UnknownResourceError;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        match uri {
            "sysinfo://cpu" => Ok(Self::Cpu),
            "sysinfo://temperature-components" => Ok(Self::TemperatureComponents),
            "sysinfo://memory" => Ok(Self::Memory),
            "sysinfo://battery" => Ok(Self::Battery),
            "sysinfo://disks" => Ok(Self::Disks),
            "sysinfo://network" => Ok(Self::Network),
            "sysinfo://uptime" => Ok(Self::Uptime),
            _ => Err(UnknownResourceError::new(uri)),
        }
    }
}

impl Display for SysinfoMcpResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}
