# Instance Types

Launcher instances come in three types, determined at load time via `InstanceType`. All share the same plugin, area, and messaging infrastructure — they differ
only in whether a GTK window is created and how widgets are rendered.

## Comparison

| Property             | GTK                                              | Headless                                          | Web                                          |
|----------------------|--------------------------------------------------|---------------------------------------------------|----------------------------------------------|
| **GTK Window**       | Yes                                              | No                                                | No (HTTP server)                             |
| **Widget rendering** | `WidgetBuilder::build_widget()` → `gtk4::Widget` | `GraphicRenderer::render_graphic()` → RGBA pixels | `WebRenderer::render_html()` → HTML fragment |
| **GTK main thread**  | Required                                         | Not required                                      | Not required                                 |
| **User interaction** | GTK events (click, touch, key)                   | External (MacroPad button press)                  | HTTP POST / WebSocket                        |
| **Output**           | GTK widget tree                                  | RGBA pixel buffer                                 | HTML page                                    |
| **State updates**    | GTK signal system                                | Re-render + `SetButtonImage`                      | WebSocket push                               |
| **PluginManager**    | Identical                                        | Identical                                         | Identical                                    |
| **AreaManager**      | Identical (GTK widgets)                          | Identical (logical areas)                         | Identical (logical areas)                    |
| **Message broker**   | Identical                                        | Identical                                         | Identical                                    |

## What Is Identical

All three instance types share the **complete plugin and area infrastructure**:

- `PluginManager` — Loads plugin shared libraries, manages plugin instances
- `AreaManager` — Manages areas and plugin-to-area mappings
- Message broker routing — `target_instance_id` routes messages equally
- Button actions — `click_topic`, `click_payload`, `click_instance` work identically
- MCP tool registration — Plugins in all instance types register tools via `RegisterToolMessage`

## GTK Instances

1. `load_instance()` calls `idle_add_local_once` → builds `ApplicationWindow` on the GTK main thread
2. `stop_instance()` calls `idle_add_local_once` → closes the window
3. Widgets render via the GTK signal system (`GtkWidget::snapshot()`)
4. User interaction arrives via GTK events

## Headless Instances

1. `load_instance()` skips window creation entirely
2. `stop_instance()` skips `window.close()` — only unloads areas
3. Widgets render via `GraphicRenderer::render_graphic(w, h)` → raw RGBA pixels
4. User interaction arrives externally (e.g. MacroPad button press via `MacroPadInputMessage`)

### Why Headless Is Necessary

GTK rendering requires an `ApplicationWindow` and a GDK surface. Without a window, there is no render context. For MacroPad devices (Stream Deck, Loupedeck),
there is no display — the device is a USB-HID keyboard with LCD keys. Button images must be sent as pixel buffers to the device driver.

### MacroPad Device Metadata

Headless MacroPad instances carry `MacroPadDeviceMetadata`:

```rust
pub struct MacroPadDeviceMetadata {
    pub device_id: String,
    pub key_count: u8,
    pub key_width: u32,
    pub key_height: u32,
    pub driver: String,
}
```

## Web Instances

1. `load_instance()` skips window creation
2. `stop_instance()` closes all WebSockets, then unloads areas
3. Widgets render via `WebRenderer::render_html()` → HTML fragments
4. The host composes the full page from a template + fragments
5. User interaction arrives via HTTP POST (`/instances/{id}/click/{plugin_id}`)
6. State updates are pushed via WebSocket as partial HTML replacements

## Configuration

Instance type is set in `instances.toml`:

```toml
[instances.side1]
instance_type = "gtk"
config_path = "configs/launcher/config.toml"

[instances.macropad_1]
instance_type = "headless"
config_path = "configs/launcher/streamdeck.toml"

[instances.web1]
instance_type = "web"
config_path = "configs/launcher/web.toml"
```

See [Multi-Instance](../features/multi-instance.md) and [MacroPad Integration](../features/macropad.md) for feature details.
