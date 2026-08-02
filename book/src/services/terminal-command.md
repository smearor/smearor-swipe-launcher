# terminal_command (Service)

Terminal command execution service for launching and managing terminal commands from widgets.

## Description

The terminal_command service launches terminal commands (e.g. `btop`, `htop`) and manages their lifecycle. It supports launching, terminating, and querying the
status of running commands.

## Topics

| Topic                              | Direction         | Description                    |
|------------------------------------|-------------------|--------------------------------|
| `service.terminal_command.command` | Widget → Service  | Launch or terminate a command  |
| `service.terminal_command.status`  | Service → Widgets | Command running/stopped status |

## Configuration Example

```toml
[btop]
main_text = "btop"
icon = "nf-md-chart_bar"
click_topic = "service.terminal_command.command"
click_payload = { action = "Launch", command_id = "btop" }
longpress_topic = "service.terminal_command.command"
longpress_payload = { action = "Terminate", command_id = "btop" }
```

## MCP Tools

| Tool                 | Description                 |
|----------------------|-----------------------------|
| `terminal_launch`    | Launch a terminal command   |
| `terminal_terminate` | Terminate a running command |

## Configuration

```toml
[[services]]
id = "terminal_command"
path = "target/release/libsmearor_terminal_command_service.so"
```

## Crate

- **Path**: `services/terminal_command/`
- **Library**: `libsmearor_terminal_command_service.so`
- **Model**: `model/terminal_command/`
