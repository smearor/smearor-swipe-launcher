# workspace-switcher (Plugin)

Visual workspace switcher widget that displays the current workspace and allows switching between workspaces.

## Description

The workspace-switcher widget communicates with the [hyprland service](../services/hyprland.md) to track workspace state. It shows the current workspace
number/name and provides visual feedback for active vs. inactive workspaces.

## Configuration

```toml
[workspace_switcher]
path = "target/release/libsmearor_workspace_switcher.so"
widget = "workspace_switcher"
icon_size = 32
icon_only = false
mode = "compact"
max_width = 200
show_scrollbar = true
default_icon = "nf-md-desktop_classic"
```

| Field            | Type                   | Description                       |
|------------------|------------------------|-----------------------------------|
| `icon_size`      | `i32`                  | Icon size in pixels               |
| `icon_only`      | `bool`                 | Show only the icon                |
| `mode`           | `WidgetMode`           | `compact` or `wide`               |
| `max_width`      | `Option<i32>`          | Maximum widget width              |
| `show_scrollbar` | `bool`                 | Show scrollbar for workspace list |
| `default_icon`   | `String`               | Default workspace icon            |
| `icon_map`       | `HashMap<i32, String>` | Map workspace IDs to icons        |

## Dynamic Icons

The icon changes based on the current workspace. An `icon_map` can map specific workspace IDs to custom icons.

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Click switches to the next workspace; scroll navigates between workspaces.

## Related Service

- [hyprland (service)](../services/hyprland.md) — Workspace tracking, window management, dispatch actions

## Crate

- **Path**: `plugins/workspace-switcher/`
- **Library**: `libsmearor_workspace_switcher.so`
- **Model**: `model/workspace/`
