# Action Bindings

Action bindings allow configuring what happens when a user interacts with a widget — without changing the widget's code. Each interaction type (click,
long-press, swipe, etc.) can be bound to a broker message with a custom topic and payload.

## Supported Binding Types

| Binding              | Description                                                  |
|----------------------|--------------------------------------------------------------|
| `click`              | Short press and release                                      |
| `longpress`          | Press held ≥ 500ms                                           |
| `hold`               | Continuous press (hold_start on press, hold_stop on release) |
| `double_press`       | Two clicks within a configurable window                      |
| `swipe_up`           | Swipe gesture upward                                         |
| `swipe_down`         | Swipe gesture downward                                       |
| `right_click`        | Right mouse button click                                     |
| `middle_click`       | Middle mouse button click                                    |
| `scroll_up`          | Scroll wheel up                                              |
| `scroll_down`        | Scroll wheel down                                            |
| `compound_longpress` | 2+ buttons in a span group held simultaneously               |
| `init`               | Action dispatched when the widget is initialized             |

## Binding Structure

Each binding specifies:

- `topic` — The broker topic to send to (e.g. `service.audio.command`)
- `payload` — A TOML inline table with the message payload
- `instance` — Optional target instance ID (defaults to the current instance)
- `mode` — `replace` (default) or `supplement`

## Binding Modes

| Mode         | Behavior                                                     |
|--------------|--------------------------------------------------------------|
| `replace`    | The binding replaces the widget's default fallback behavior  |
| `supplement` | Both the binding **and** the default fallback are dispatched |

`supplement` mode allows configuring a click binding that sends a message while still performing the widget's default action (e.g. launching an app).

## Configuration Example

```toml
[my_button]
defaults = "menu_button"
main_text = "Volume Up"
icon = "nf-md-volume_high"

click_topic = "service.audio.command"
click_payload = { action = "VolumeUp", step = 5 }
click_mode = "replace"

longpress_topic = "service.audio.command"
longpress_payload = { action = "ToggleMute" }
longpress_mode = "supplement"
```

## Defaults Templates

Bindings can inherit from default templates defined in `[defaults.*]` sections:

```toml
[defaults.menu_button]
click_topic = "area.open"
longpress_topic = "area.close"
enabled = true
css_classes = ["menu-button"]
```

Widgets reference a template with `defaults = "menu_button"`. Instance-specific values always override the template.

## Widget Support

All widgets that use the shared `ActionBindings` struct from `plugin-api` support all binding types. This includes: `button`, `audio`, `mpris`, `weather`,
`clock`, `power`, `wallpaper`, `network`, `app-launcher`, `voice_assistant`, `workspace-switcher`, `notifications`, and `sysinfo`.

See [Using Action Bindings](../plugin-api/action-bindings.md) for the API perspective.
