# hyprland (Service)

Hyprland compositor integration service for workspace tracking, window management, and dispatch actions.

## Description

The hyprland service connects to the Hyprland IPC socket and tracks workspace changes, window events, and compositor state. It provides dispatch actions for
workspace switching, window management, and special workspace toggles. It broadcasts compositor events to all instances.

## Topics

| Topic                           | Direction         | Description                      |
|---------------------------------|-------------------|----------------------------------|
| `service.hyprland.dispatch`     | Widget → Service  | Dispatch a Hyprland action       |
| `service.hyprland.status`       | Service → Widgets | Workspace / window state updates |
| `compositor::workspace_changed` | Service → All     | Workspace change broadcast       |
| `compositor::monitor_changed`   | Service → All     | Monitor configuration change     |

## Dispatch Actions

| Action                   | Description                                  |
|--------------------------|----------------------------------------------|
| `Workspace`              | Switch to a workspace (absolute or relative) |
| `MoveToWorkspace`        | Move active window to a workspace            |
| `ToggleFloating`         | Toggle floating mode for active window       |
| `ToggleFullscreen`       | Toggle fullscreen mode                       |
| `KillActiveWindow`       | Close the active window                      |
| `ToggleSpecialWorkspace` | Toggle a special workspace                   |

## MCP Tools

| Tool                         | Description                |
|------------------------------|----------------------------|
| `hyprland_switch_workspace`  | Switch to a workspace      |
| `hyprland_move_window`       | Move window to a workspace |
| `hyprland_toggle_floating`   | Toggle floating mode       |
| `hyprland_toggle_fullscreen` | Toggle fullscreen          |
| `hyprland_kill_window`       | Kill active window         |

## Configuration

```toml
[[services]]
id = "hyprland"
path = "target/release/libsmearor_hyprland_service.so"
```

## Crate

- **Path**: `services/hyprland/`
- **Library**: `libsmearor_hyprland_service.so`
- **Model**: `model/hyprland/`, `model/hyprland-shared/`, `model/hyprland-dispatch/`, `model/hyprland-status/`, `model/hyprland-command/`
