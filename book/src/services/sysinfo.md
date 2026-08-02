# sysinfo (Service)

System information collection service that gathers CPU, memory, disk, network, temperature, uptime, and load metrics.

## Description

The sysinfo service uses the `sysinfo` crate to periodically collect system metrics. It broadcasts updates to all [sysinfo widgets](../plugins/sysinfo.md) and
provides on-demand queries via MCP tools.

## Topics

| Topic                     | Direction         | Description             |
|---------------------------|-------------------|-------------------------|
| `service.sysinfo.status`  | Service → Widgets | Periodic metrics update |
| `service.sysinfo.command` | Widget → Service  | Query specific metrics  |

## MCP Tools

| Tool                      | Description              |
|---------------------------|--------------------------|
| `sysinfo_get_cpu`         | Get CPU usage            |
| `sysinfo_get_memory`      | Get memory usage         |
| `sysinfo_get_disk`        | Get disk usage           |
| `sysinfo_get_network`     | Get network I/O          |
| `sysinfo_get_temperature` | Get temperature readings |
| `sysinfo_get_uptime`      | Get system uptime        |
| `sysinfo_get_load`        | Get system load average  |

## Configuration

```toml
[[services]]
id = "sysinfo"
path = "target/release/libsmearor_sysinfo_service.so"
```

## Crate

- **Path**: `services/sysinfo/`
- **Library**: `libsmearor_sysinfo_service.so`
- **Model**: `model/sysinfo/`
