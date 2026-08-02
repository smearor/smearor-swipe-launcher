# hyprland (Service)

Hyprland compositor integration service for workspace tracking, window management, and dispatch actions.

## Description

The hyprland service connects to the Hyprland IPC socket and tracks workspace changes, window events, and compositor state. It provides dispatch actions for
workspace switching, window management, and special workspace toggles. It broadcasts compositor events to all instances.

## Topics

| Topic                                 | Direction         | Description                         |
|---------------------------------------|-------------------|-------------------------------------|
| `service.hyprland.dispatch.workspace` | Widget → Service  | Dispatch a workspace-related action |
| `service.hyprland.dispatch.toggle`    | Widget → Service  | Dispatch a toggle-related action    |
| `service.hyprland.dispatch.window`    | Widget → Service  | Dispatch a window-related action    |
| `service.hyprland.dispatch.system`    | Widget → Service  | Dispatch a system-related action    |
| `service.hyprland.status`             | Service → Widgets | Workspace / window state updates    |
| `compositor::workspace_changed`       | Service → All     | Workspace change broadcast          |
| `compositor::monitor_changed`         | Service → All     | Monitor configuration change        |

## Dispatch Messages

Dispatch messages use a `kind` + `ops` payload structure. The `kind` field selects the action, and `ops` carries optional action-specific parameters. When `ops`
is omitted, `Default::default()` is used.

### Workspace Dispatch

```toml
click_topic = "service.hyprland.dispatch.workspace"
click_payload = { kind = "Workspace", ops = { workspace = { identifier = { kind = "Relative", id = -1 } } } }
```

| Kind              | Ops Field           | Description                                  |
|-------------------|---------------------|----------------------------------------------|
| `Workspace`       | `workspace`         | Switch to a workspace (absolute or relative) |
| `MoveToWorkspace` | `move_to_workspace` | Move active window to a workspace            |

### Toggle Dispatch

```toml
click_topic = "service.hyprland.dispatch.toggle"
click_payload = { kind = "ToggleFloating" }
```

| Kind                   | Ops Field           | Description                                                          |
|------------------------|---------------------|----------------------------------------------------------------------|
| `ToggleFloating`       | —                   | Toggle floating mode (no ops needed)                                 |
| `ToggleFullscreen`     | `toggle_fullscreen` | Toggle fullscreen (`fullscreen_type`: `Real`, `Maximize`, `NoParam`) |
| `ToggleFakeFullscreen` | —                   | Toggle fake fullscreen                                               |
| `ToggleGroup`          | —                   | Toggle window group                                                  |
| `ToggleDpms`           | `toggle_dpms`       | Toggle display power management                                      |

### Window Dispatch

```toml
click_topic = "service.hyprland.dispatch.window"
click_payload = { kind = "KillActiveWindow" }
```

| Kind               | Ops Field       | Description                         |
|--------------------|-----------------|-------------------------------------|
| `KillActiveWindow` | —               | Close the active window             |
| `CenterWindow`     | —               | Center the active window            |
| `MoveFocus`        | `move_focus`    | Move focus in a direction           |
| `MoveWindow`       | `move_window`   | Move active window                  |
| `FocusWindow`      | `focus_window`  | Focus a specific window             |
| `FocusMonitor`     | `focus_monitor` | Focus a specific monitor            |
| `FocusMaster`      | `focus_master`  | Focus the master window             |
| `CycleWindow`      | `cycle_window`  | Cycle focus to next/previous window |
| `ResizeActive`     | `resize_active` | Resize the active window            |
| `MoveCursor`       | `move_cursor`   | Move cursor to coordinates          |

### System Dispatch

```toml
click_topic = "service.hyprland.dispatch.system"
click_payload = { kind = "Exit" }
```

| Kind                  | Ops Field     | Description                      |
|-----------------------|---------------|----------------------------------|
| `Exit`                | —             | Exit the Hyprland compositor     |
| `ForceRendererReload` | —             | Force renderer reload            |
| `AddMaster`           | `add_master`  | Add a master window              |
| `OrientationCenter`   | —             | Set orientation to center        |
| `OrientationLeft`     | —             | Set orientation to left          |
| `OrientationRight`    | —             | Set orientation to right         |
| `OrientationTop`      | —             | Set orientation to top           |
| `OrientationPrev`     | —             | Set orientation to previous      |
| `MoveOutOfGroup`      | —             | Move active window out of group  |
| `LockGroups`          | `lock_groups` | Lock/unlock/toggle window groups |
| `Pass`                | `pass`        | Pass key press to a window       |

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
