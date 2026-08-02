# button (Plugin)

A generic, configurable button widget. The most versatile plugin — it displays an icon, text, and info text, and dispatches configurable broker messages on user
interaction.

## Description

The button widget is the workhorse of the launcher. It reads its entire behavior from configuration: icon, text, colors, CSS classes, and action bindings. It
has no default actions — all behavior is configured via [action bindings](../features/action-bindings.md).

## Configuration

```toml
[my_button]
defaults = "menu_button"
main_text = "My Button"
info_text = "Description"
icon = "nf-md-star"
icon_color = "#dc0073ff"
tooltip = "My Button"
css_classes = ["menu-button", "glow-blue"]
click_topic = "area.open"
click_payload = { area_id = "my_area" }
longpress_topic = "area.close"
longpress_payload = { area_id = "my_area" }
```

| Field             | Type             | Description                           |
|-------------------|------------------|---------------------------------------|
| `main_text`       | `String`         | Primary text label                    |
| `info_text`       | `String`         | Secondary text label                  |
| `icon`            | `Option<String>` | Nerd Font or GTK icon name            |
| `icon_size`       | `i32`            | Icon size in pixels                   |
| `icon_only`       | `bool`           | Show only the icon                    |
| `icon_color`      | `Option<String>` | Hex color for the icon                |
| `main_text_color` | `Option<String>` | Hex color for main text               |
| `info_text_color` | `Option<String>` | Hex color for info text               |
| `tooltip`         | `Option<String>` | Tooltip text                          |
| `css_classes`     | `Vec<String>`    | CSS classes to apply                  |
| `enabled`         | `bool`           | Whether the button is enabled         |
| `active`          | `bool`           | Whether the button is in active state |
| `state_topic`     | `Option<String>` | Topic to listen for state updates     |
| `state_icon`      | `Option<String>` | Icon to show when state is active     |

## State-Dependent Icons

The button can listen to a `state_topic` and switch between its default icon and `state_icon` based on the state message payload.

## Defaults Templates

Buttons commonly inherit from `[defaults.menu_button]` or `[defaults.close_button]` templates to avoid repeating common configuration.

## Action Bindings

Supports all [action binding types](../features/action-bindings.md). Since the button has no default fallback, `supplement` mode effectively only dispatches the
binding.

## Examples

### Hyprland Dispatch

Buttons can trigger [Hyprland dispatch actions](../services/hyprland.md) via subtopics with `kind` + `ops` payloads:

```toml
[toggle_floating]
defaults = "menu_button"
main_text = "Toggle Floating"
icon = "nf-md-window_restore"
click_topic = "service.hyprland.dispatch.toggle"
click_payload = { kind = "ToggleFloating" }

[previous_workspace]
defaults = "menu_button"
main_text = "Previous"
icon = "nf-md-skip_previous"
click_topic = "service.hyprland.dispatch.workspace"
click_payload = { kind = "Workspace", ops = { workspace = { identifier = { kind = "Relative", id = -1 } } } }
longpress_topic = "service.hyprland.dispatch.workspace"
longpress_payload = { kind = "MoveToWorkspace", ops = { move_to_workspace = { identifier = { kind = "RelativeOpen", id = -1 } } } }
```

## Crate

- **Path**: `plugins/button/`
- **Library**: `libsmearor_button_widget.so`
