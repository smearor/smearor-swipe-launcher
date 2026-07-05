/// Topic for CPU metrics, including usage and temperature.
pub const TOPIC_CPU: &str = "service.sysinfo.cpu.status";

/// Topic for memory metrics, including usage and absolute values.
pub const TOPIC_MEMORY: &str = "service.sysinfo.memory.status";

/// Topic for battery metrics, including level and charging state.
pub const TOPIC_BATTERY: &str = "service.sysinfo.battery.status";

/// Topic for disk metrics, including per-mount usage and throughput.
pub const TOPIC_DISKS: &str = "service.sysinfo.disks.status";

/// Topic for network metrics, including inbound and outbound throughput.
pub const TOPIC_NETWORK: &str = "service.sysinfo.network.status";

/// Topic for uptime and load average metrics.
pub const TOPIC_UPTIME: &str = "service.sysinfo.uptime.status";
