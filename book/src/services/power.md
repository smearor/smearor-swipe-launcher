# power (Service)

Power management service using systemd-logind for shutdown, reboot, suspend, hibernate, lock, and logout.

## Description

The power service communicates with `systemd-logind` via D-Bus to execute power actions. It also checks for power inhibitors (e.g. unsaved work) before
executing destructive actions.

## Topics

| Topic                   | Direction         | Description                     |
|-------------------------|-------------------|---------------------------------|
| `service.power.command` | Widget → Service  | Shutdown, reboot, suspend, etc. |
| `service.power.status`  | Service → Widgets | Inhibitor state, action result  |

## MCP Tools

| Tool              | Description            |
|-------------------|------------------------|
| `power_shutdown`  | Power off the system   |
| `power_reboot`    | Restart the system     |
| `power_suspend`   | Suspend to RAM         |
| `power_hibernate` | Suspend to disk        |
| `power_lock`      | Lock the screen        |
| `power_logout`    | Log out of the session |

## Configuration

```toml
[[services]]
id = "power"
path = "target/release/libsmearor_power_service.so"
```

## Crate

- **Path**: `services/power/`
- **Library**: `libsmearor_power_service.so`
- **Model**: `model/power/`
