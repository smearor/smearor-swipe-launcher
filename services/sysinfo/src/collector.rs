use smearor_sysinfo_model::BatteryStatus;
use smearor_sysinfo_model::BatteryStatusMessage;
use smearor_sysinfo_model::CpuStatusMessage;
use smearor_sysinfo_model::DiskUsage;
use smearor_sysinfo_model::DisksStatusMessage;
use smearor_sysinfo_model::MemoryStatusMessage;
use smearor_sysinfo_model::NetworkStatusMessage;
use smearor_sysinfo_model::TemperatureComponent;
use smearor_sysinfo_model::UptimeStatusMessage;
use std::io;
use std::path::Path;
use std::time::Instant;
use tokio::fs;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// Error type for metric collection failures.
#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    /// Failed to read a system file.
    #[error("failed to read {path}: {source}")]
    Read { path: String, source: io::Error },
    /// Failed to parse a value from a system file.
    #[error("failed to parse {path}: {reason}")]
    Parse { path: String, reason: String },
}

/// Collector state for delta-based metrics.
pub struct CollectorState {
    /// Previous disk sample for throughput calculation.
    pub disk_last_sample: Option<DiskSample>,
    /// Previous network sample for throughput calculation.
    pub network_last_sample: Option<NetworkSample>,
    /// Previous CPU sample for usage calculation.
    pub cpu_last_sample: Option<CpuSample>,
}

impl Default for CollectorState {
    fn default() -> Self {
        Self {
            disk_last_sample: None,
            network_last_sample: None,
            cpu_last_sample: None,
        }
    }
}

/// Raw disk counters for a single device.
pub(self) struct DiskSample {
    timestamp: Instant,
    read_sectors: u64,
    write_sectors: u64,
}

/// Raw network counters for all interfaces.
pub(self) struct NetworkSample {
    timestamp: Instant,
    received_bytes: u64,
    transmitted_bytes: u64,
}

/// Raw CPU tick counters.
pub(self) struct CpuSample {
    idle: u64,
    total: u64,
}

/// Collects CPU metrics.
///
/// `temperature_component_filter` selects which component's temperature is used
/// as the primary `cpu_temperature`. If `None`, the first available component is used.
pub async fn collect_cpu(
    enable_temperature: bool,
    temperature_source: Option<&str>,
    temperature_component_filter: Option<&str>,
    state: &mut CollectorState,
) -> CpuStatusMessage {
    let usage = read_cpu_usage(state).await.unwrap_or_else(|error| {
        error!("CPU collector failed: {error}");
        0.0
    });

    let (temperature, components) = if enable_temperature {
        let components = read_temperature_components(temperature_source).await;
        if components.is_empty() {
            debug!("CPU temperature not available: no temperature components found");
            (stabby::option::Option::None(), components)
        } else {
            let selected = select_temperature_component(&components, temperature_component_filter);
            match selected {
                Some(comp) => {
                    let temp = comp.temperature.clone();
                    (temp, components)
                }
                None => {
                    debug!("CPU temperature not available: no component matched filter '{:?}'", temperature_component_filter);
                    (stabby::option::Option::None(), components)
                }
            }
        }
    } else {
        (stabby::option::Option::None(), stabby::vec::Vec::new())
    };

    CpuStatusMessage::new(usage, temperature, components)
}

/// Collects memory metrics.
pub async fn collect_memory() -> MemoryStatusMessage {
    read_memory().await.unwrap_or_else(|error| {
        error!("Memory collector failed: {error}");
        MemoryStatusMessage::default()
    })
}

/// Collects battery metrics.
pub async fn collect_battery() -> BatteryStatusMessage {
    read_battery().await.unwrap_or_else(|error| {
        trace!("Battery collector failed: {error}");
        BatteryStatusMessage::default()
    })
}

/// Collects disk metrics.
pub async fn collect_disks(state: &mut CollectorState) -> DisksStatusMessage {
    read_disks(state).await.unwrap_or_else(|error| {
        error!("Disk collector failed: {error}");
        DisksStatusMessage::default()
    })
}

/// Collects network metrics.
pub async fn collect_network(state: &mut CollectorState) -> NetworkStatusMessage {
    read_network(state).await.unwrap_or_else(|error| {
        error!("Network collector failed: {error}");
        NetworkStatusMessage::default()
    })
}

/// Collects uptime and load average metrics.
pub async fn collect_uptime() -> UptimeStatusMessage {
    read_uptime().await.unwrap_or_else(|error| {
        error!("Uptime collector failed: {error}");
        UptimeStatusMessage::default()
    })
}

async fn read_cpu_usage(state: &mut CollectorState) -> Result<f32, CollectorError> {
    let content = read_file("/proc/stat").await?;
    let line = content.lines().find(|line| line.starts_with("cpu ")).ok_or_else(|| CollectorError::Parse {
        path: String::from("/proc/stat"),
        reason: String::from("missing aggregate cpu line"),
    })?;

    let values: Vec<u64> = line.split_whitespace().skip(1).filter_map(|token| token.parse().ok()).collect();

    if values.len() < 4 {
        return Err(CollectorError::Parse {
            path: String::from("/proc/stat"),
            reason: String::from("not enough cpu values"),
        });
    }

    let idle = values[3];
    let total: u64 = values.iter().sum();
    let current = CpuSample { idle, total };

    let usage = if let Some(previous) = state.cpu_last_sample.take() {
        let total_delta = current.total.saturating_sub(previous.total);
        let idle_delta = current.idle.saturating_sub(previous.idle);
        if total_delta == 0 {
            0.0
        } else {
            let busy = total_delta.saturating_sub(idle_delta);
            (busy as f32 / total_delta as f32) * 100.0
        }
    } else {
        0.0
    };

    state.cpu_last_sample = Some(current);
    Ok(usage)
}

/// Reads all available temperature components using `sysinfo` first, falling back
/// to the `/sys`-based approach if `sysinfo` returns no components.
async fn read_temperature_components(source: Option<&str>) -> stabby::vec::Vec<TemperatureComponent> {
    let components = read_temperature_components_sysinfo().await;
    if !components.is_empty() {
        return components;
    }

    debug!("sysinfo returned no temperature components, falling back to /sys approach");
    read_temperature_components_sysfs(source).await
}

/// Reads temperature components using the `sysinfo` crate.
async fn read_temperature_components_sysinfo() -> stabby::vec::Vec<TemperatureComponent> {
    let sysinfo_components = sysinfo::Components::new_with_refreshed_list();
    let mut components = stabby::vec::Vec::new();
    for component in &sysinfo_components {
        let label = component.label();
        let id = component.id().unwrap_or("");
        let temperature = component.temperature().filter(|t| t.is_finite() && *t > 0.0);
        let max = component.max().filter(|t| t.is_finite() && *t > 0.0);
        let critical = component.critical().filter(|t| t.is_finite() && *t > 0.0);
        components.push(TemperatureComponent::new(
            label,
            id,
            temperature.map(stabby::option::Option::Some).unwrap_or(stabby::option::Option::None()),
            max.map(stabby::option::Option::Some).unwrap_or(stabby::option::Option::None()),
            critical.map(stabby::option::Option::Some).unwrap_or(stabby::option::Option::None()),
        ));
    }
    components
}

/// Reads temperature components using the `/sys` filesystem as fallback.
async fn read_temperature_components_sysfs(source: Option<&str>) -> stabby::vec::Vec<TemperatureComponent> {
    match source {
        None | Some("thermal_zone") => read_temperature_components_thermal_zone().await,
        Some("hwmon") => read_temperature_components_hwmon().await,
        Some(path) => read_temperature_components_path(Path::new(path)).await,
    }
}

/// Selects a temperature component by label or id match.
///
/// If `filter` is `None`, returns the first component with a valid temperature.
/// If `filter` is `Some`, returns the first component whose label or id contains the filter string.
fn select_temperature_component<'a>(components: &'a [TemperatureComponent], filter: Option<&str>) -> Option<&'a TemperatureComponent> {
    match filter {
        None => components.iter().find(|c| c.temperature.is_some()),
        Some(filter_str) => components.iter().find(|c| {
            (c.label.to_string().to_lowercase().contains(&filter_str.to_lowercase()) || c.id.to_string().to_lowercase().contains(&filter_str.to_lowercase()))
                && c.temperature.is_some()
        }),
    }
}

async fn read_temperature_components_thermal_zone() -> stabby::vec::Vec<TemperatureComponent> {
    let thermal_dir = Path::new("/sys/class/thermal");
    let mut entries = match fs::read_dir(thermal_dir).await {
        Ok(entries) => entries,
        Err(error) => {
            debug!("Failed to read /sys/class/thermal: {error}");
            return stabby::vec::Vec::new();
        }
    };

    let mut components = stabby::vec::Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let temp_path = path.join("temp");
        if !temp_path.exists() {
            continue;
        }
        let zone_name = entry.file_name().to_string_lossy().to_string();
        let content = match read_file(&temp_path).await {
            Ok(content) => content,
            Err(error) => {
                debug!("Failed to read {:?}: {error}", temp_path);
                continue;
            }
        };
        if let Ok(millis) = content.trim().parse::<i32>() {
            if millis > 0 {
                components.push(TemperatureComponent::new(
                    zone_name.as_str(),
                    zone_name.as_str(),
                    stabby::option::Option::Some(millis as f32 / 1000.0),
                    stabby::option::Option::None(),
                    stabby::option::Option::None(),
                ));
            }
        }
    }
    components
}

async fn read_temperature_components_hwmon() -> stabby::vec::Vec<TemperatureComponent> {
    let hwmon_dir = Path::new("/sys/class/hwmon");
    let mut entries = match fs::read_dir(hwmon_dir).await {
        Ok(entries) => entries,
        Err(error) => {
            debug!("Failed to read /sys/class/hwmon: {error}");
            return stabby::vec::Vec::new();
        }
    };

    let mut components = stabby::vec::Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let hwmon_name = entry.file_name().to_string_lossy().to_string();

        let name_content = read_file(&path.join("name")).await.ok();
        let device_name = name_content.map(|c| c.trim().to_string()).unwrap_or_else(|| hwmon_name.clone());

        let mut temp_entries = match fs::read_dir(&path).await {
            Ok(entries) => entries,
            Err(error) => {
                debug!("Failed to read hwmon dir {:?}: {error}", path);
                continue;
            }
        };
        while let Ok(Some(temp_entry)) = temp_entries.next_entry().await {
            let name = temp_entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("temp") && name_str.ends_with("_input") {
                let temp_path = temp_entry.path();
                if let Ok(content) = read_file(&temp_path).await {
                    if let Ok(millis) = content.trim().parse::<i32>() {
                        if millis > 0 {
                            let label = format!("{} {}", device_name, name_str);
                            let id = format!("{}_{}", hwmon_name, name_str.trim_end_matches("_input"));
                            components.push(TemperatureComponent::new(
                                label.as_str(),
                                id.as_str(),
                                stabby::option::Option::Some(millis as f32 / 1000.0),
                                stabby::option::Option::None(),
                                stabby::option::Option::None(),
                            ));
                        }
                    }
                }
            }
        }
    }
    components
}

async fn read_temperature_components_path(path: &Path) -> stabby::vec::Vec<TemperatureComponent> {
    let content = match read_file(path).await {
        Ok(content) => content,
        Err(error) => {
            debug!("Failed to read {:?}: {error}", path);
            return stabby::vec::Vec::new();
        }
    };
    if let Ok(millis) = content.trim().parse::<i32>() {
        if millis > 0 {
            let path_str = path.to_string_lossy().to_string();
            let mut components = stabby::vec::Vec::new();
            components.push(TemperatureComponent::new(
                path_str.as_str(),
                path_str.as_str(),
                stabby::option::Option::Some(millis as f32 / 1000.0),
                stabby::option::Option::None(),
                stabby::option::Option::None(),
            ));
            return components;
        }
    }
    stabby::vec::Vec::new()
}

async fn read_memory() -> Result<MemoryStatusMessage, CollectorError> {
    let content = read_file("/proc/meminfo").await?;
    let mut total: Option<u64> = None;
    let mut available: Option<u64> = None;

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next();
        let value = parts.next().and_then(|token| token.parse::<u64>().ok());
        match key {
            Some("MemTotal:") => total = value.map(|v| v * 1024),
            Some("MemAvailable:") => available = value.map(|v| v * 1024),
            _ => {}
        }
    }

    let total_kb = total.ok_or_else(|| CollectorError::Parse {
        path: String::from("/proc/meminfo"),
        reason: String::from("missing MemTotal"),
    })?;
    let available_kb = available.ok_or_else(|| CollectorError::Parse {
        path: String::from("/proc/meminfo"),
        reason: String::from("missing MemAvailable"),
    })?;

    let used = total_kb.saturating_sub(available_kb);
    let usage = if total_kb == 0 { 0.0 } else { (used as f32 / total_kb as f32) * 100.0 };

    Ok(MemoryStatusMessage::new(usage, total_kb, used, available_kb))
}

async fn read_battery() -> Result<BatteryStatusMessage, CollectorError> {
    let power_dir = Path::new("/sys/class/power_supply");
    let mut entries = fs::read_dir(power_dir).await.map_err(|source| CollectorError::Read {
        path: String::from("/sys/class/power_supply"),
        source,
    })?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("BAT") {
            continue;
        }
        let path = entry.path();
        let capacity_path = path.join("capacity");
        let status_path = path.join("status");
        if !capacity_path.exists() {
            continue;
        }

        let capacity_content = read_file(&capacity_path).await?;
        let level: f32 = capacity_content.trim().parse().map_err(|_| CollectorError::Parse {
            path: capacity_path.to_string_lossy().to_string(),
            reason: String::from("invalid capacity value"),
        })?;

        let status = if status_path.exists() {
            let status_content = read_file(&status_path).await?;
            match status_content.trim() {
                "Charging" => BatteryStatus::Charging,
                "Discharging" => BatteryStatus::Discharging,
                "Full" => BatteryStatus::Full,
                _ => BatteryStatus::Unknown,
            }
        } else {
            BatteryStatus::Unknown
        };

        return Ok(BatteryStatusMessage::new(level, status));
    }

    Err(CollectorError::Parse {
        path: String::from("/sys/class/power_supply"),
        reason: String::from("no battery found"),
    })
}

async fn read_disks(state: &mut CollectorState) -> Result<DisksStatusMessage, CollectorError> {
    let mut mounts = stabby::vec::Vec::new();
    let mount_content = read_file("/proc/mounts").await?;

    for line in mount_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let mount_point = parts[1];
        let filesystem = parts[2];
        if filesystem == "devtmpfs"
            || filesystem == "proc"
            || filesystem == "sysfs"
            || filesystem == "tmpfs"
            || filesystem == "devpts"
            || filesystem == "cgroup"
            || filesystem.starts_with("fuse")
        {
            continue;
        }
        if let Ok(stat) = nix::sys::statfs::statfs(mount_point) {
            let block_size = stat.block_size() as u64;
            let total_blocks = stat.blocks();
            let available_blocks = stat.blocks_available();
            let used_blocks = total_blocks.saturating_sub(available_blocks);
            let total = total_blocks * block_size;
            let used = used_blocks * block_size;
            let available = available_blocks * block_size;
            let usage = if total == 0 { 0.0 } else { (used as f32 / total as f32) * 100.0 };
            mounts.push(DiskUsage {
                mount_point: stabby::string::String::from(mount_point),
                usage,
                total,
                used,
                available,
            });
        }
    }

    let (read_sectors, write_sectors) = read_diskstats().await?;
    let now = Instant::now();
    let (read_bytes_per_second, write_bytes_per_second) = if let Some(previous) = state.disk_last_sample.take() {
        let duration = now.duration_since(previous.timestamp).as_secs_f64();
        let read_delta = read_sectors.saturating_sub(previous.read_sectors);
        let write_delta = write_sectors.saturating_sub(previous.write_sectors);
        if duration > 0.0 {
            ((read_delta * 512) as f64 / duration, (write_delta * 512) as f64 / duration)
        } else {
            (0.0, 0.0)
        }
    } else {
        (0.0, 0.0)
    };

    state.disk_last_sample = Some(DiskSample {
        timestamp: now,
        read_sectors,
        write_sectors,
    });

    Ok(DisksStatusMessage::new(mounts, read_bytes_per_second as u64, write_bytes_per_second as u64))
}

async fn read_diskstats() -> Result<(u64, u64), CollectorError> {
    let content = read_file("/proc/diskstats").await?;
    let mut read_sectors = 0u64;
    let mut write_sectors = 0u64;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 14 {
            continue;
        }
        let device = parts[2];
        if device.starts_with("loop") || device.starts_with("ram") || device.starts_with("dm-") {
            continue;
        }
        if let (Ok(read), Ok(write)) = (parts[5].parse::<u64>(), parts[9].parse::<u64>()) {
            read_sectors += read;
            write_sectors += write;
        }
    }

    Ok((read_sectors, write_sectors))
}

async fn read_network(state: &mut CollectorState) -> Result<NetworkStatusMessage, CollectorError> {
    let content = read_file("/proc/net/dev").await?;
    let mut received_bytes = 0u64;
    let mut transmitted_bytes = 0u64;

    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }
        let interface = parts[0].trim_end_matches(':');
        if interface == "lo"
            || interface.starts_with("virbr")
            || interface.starts_with("docker")
            || interface.starts_with("veth")
            || interface.starts_with("br-")
        {
            continue;
        }
        if let (Ok(received), Ok(transmitted)) = (parts[1].parse::<u64>(), parts[9].parse::<u64>()) {
            received_bytes += received;
            transmitted_bytes += transmitted;
        }
    }

    let now = Instant::now();
    let (received_per_second, transmitted_per_second) = if let Some(previous) = state.network_last_sample.take() {
        let duration = now.duration_since(previous.timestamp).as_secs_f64();
        let received_delta = received_bytes.saturating_sub(previous.received_bytes);
        let transmitted_delta = transmitted_bytes.saturating_sub(previous.transmitted_bytes);
        if duration > 0.0 {
            (received_delta as f64 / duration, transmitted_delta as f64 / duration)
        } else {
            (0.0, 0.0)
        }
    } else {
        (0.0, 0.0)
    };

    state.network_last_sample = Some(NetworkSample {
        timestamp: now,
        received_bytes,
        transmitted_bytes,
    });

    Ok(NetworkStatusMessage::new(received_per_second as u64, transmitted_per_second as u64))
}

async fn read_uptime() -> Result<UptimeStatusMessage, CollectorError> {
    let content = read_file("/proc/uptime").await?;
    let uptime_seconds = content
        .split_whitespace()
        .next()
        .and_then(|token| token.parse::<f64>().ok())
        .map(|value| value as u64)
        .ok_or_else(|| CollectorError::Parse {
            path: String::from("/proc/uptime"),
            reason: String::from("invalid uptime value"),
        })?;

    let load_content = read_file("/proc/loadavg").await?;
    let parts: Vec<&str> = load_content.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(CollectorError::Parse {
            path: String::from("/proc/loadavg"),
            reason: String::from("not enough load average values"),
        });
    }

    let load_1 = parts[0].parse::<f32>().map_err(|_| CollectorError::Parse {
        path: String::from("/proc/loadavg"),
        reason: String::from("invalid 1-minute load average"),
    })?;
    let load_5 = parts[1].parse::<f32>().map_err(|_| CollectorError::Parse {
        path: String::from("/proc/loadavg"),
        reason: String::from("invalid 5-minute load average"),
    })?;
    let load_15 = parts[2].parse::<f32>().map_err(|_| CollectorError::Parse {
        path: String::from("/proc/loadavg"),
        reason: String::from("invalid 15-minute load average"),
    })?;

    Ok(UptimeStatusMessage::new(uptime_seconds, load_1, load_5, load_15))
}

async fn read_file(path: impl AsRef<Path>) -> Result<String, CollectorError> {
    let path = path.as_ref();
    fs::read_to_string(path).await.map_err(|source| CollectorError::Read {
        path: path.to_string_lossy().to_string(),
        source,
    })
}
