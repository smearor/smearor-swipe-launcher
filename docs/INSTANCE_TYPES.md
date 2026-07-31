# Instance Types: Gtk, Headless, and Web

Launcher instances come in three types, determined at load time via `InstanceType`. All share the same plugin, area, and messaging infrastructure — they differ
only in whether a GTK window is created and how widgets are rendered.

---

## 1. Comparison

| Property                                               | `InstanceType::Gtk`                                     | `InstanceType::Headless`                                               | `InstanceType::Web`                                            |
|--------------------------------------------------------|---------------------------------------------------------|------------------------------------------------------------------------|----------------------------------------------------------------|
| **GTK Window**                                         | Yes — `ApplicationWindow` is built                      | No — no window, no GDK surface                                         | No — no window, served via HTTP                                |
| **`build_window()`**                                   | Called via `idle_add_local_once`                        | Skipped                                                                | Skipped                                                        |
| **Widget rendering**                                   | GTK `WidgetBuilder::build_widget()` → `gtk4::Widget`    | `GraphicRenderer::render_graphic()` → `FfiGraphic` (RGBA pixel buffer) | `WebRenderer::render_html()` → `FfiString` (HTML fragment)     |
| **GTK main thread**                                    | All widget operations must run on the GLib main context | No GTK dependency, no main-thread requirement                          | No GTK dependency, no main-thread requirement                  |
| **User interaction**                                   | GTK events (click, touch, key)                          | External input (e.g. hardware button press via `MacroPadInputMessage`) | HTTP POST / WebSocket                                          |
| **Output**                                             | GTK widget tree                                         | RGBA pixel buffer                                                      | HTML page (composed from template + fragments)                 |
| **State updates**                                      | GTK signal system                                       | Re-render + `SetButtonImage`                                           | WebSocket push (partial HTML update)                           |
| **`PluginManager`**                                    | Identical                                               | Identical                                                              | Identical                                                      |
| **`AreaManager`**                                      | Identical (with GTK widgets)                            | Identical (logical areas, no GTK widgets)                              | Identical (logical areas, no GTK widgets)                      |
| **Message broker routing**                             | Identical (`target_instance_id`)                        | Identical                                                              | Identical                                                      |
| **`click_topic` / `click_payload` / `click_instance`** | Identical                                               | Identical                                                              | Identical                                                      |
| **MCP tool registration**                              | Plugins register tools via `RegisterToolMessage`        | Plugins register tools via `RegisterToolMessage`                       | Plugins register tools via `RegisterToolMessage`               |
| **Persistence**                                        | `instances.toml` with `instance_type = "gtk"`           | `instances.toml` with `instance_type = "headless"`                     | `instances.toml` with `instance_type = "web"`                  |
| **Lifecycle**                                          | `load_instance()` → `stop_instance()`                   | `load_instance()` → `stop_instance()`                                  | `load_instance()` → `stop_instance()` (also closes WebSockets) |

---

## 2. What Is Identical

All three instance types share the **complete plugin and area infrastructure**:

- **`PluginManager`**: Loads plugin shared libraries, manages plugin instances.
- **`AreaManager`**: Manages areas and plugin-to-area mappings.
- **Message broker routing**: `target_instance_id` routes messages to all instance types equally.
- **Button actions**: `click_topic`, `click_payload`, and `click_instance` work identically.
- **MCP tool registration**: Plugins in all instance types register tools via `RegisterToolMessage`.
- **`stop_instance()` cleanup**: Plugin unload, MCP tool deregistration, and area removal are identical.

---

## 3. What Is Different

### 3.1 Gtk

1. `load_instance()` calls `idle_add_local_once` → builds `ApplicationWindow` on the GTK main thread.
2. `stop_instance()` calls `idle_add_local_once` → closes the window on the GTK main thread.
3. Widgets render themselves via the GTK signal system (`GtkWidget::snapshot()`).
4. User interaction (click, touch) arrives via GTK events.

### 3.2 Headless

1. `load_instance()` skips the `idle_add_local_once` block entirely — no window is built.
2. `stop_instance()` skips `window.close()` — only unloads areas directly.
3. Widgets must be rendered via `GraphicRenderer::render_graphic(w, h)` → produces raw RGBA pixels.
4. User interaction arrives externally (e.g. hardware button press via `MacroPadInputMessage`).

### 3.3 Web

1. `load_instance()` skips the `idle_add_local_once` block entirely — no window is built.
2. `stop_instance()` closes all WebSockets for the instance, then unloads areas directly.
3. Widgets must be rendered via `WebRenderer::render_html(instance_id, plugin_id)` → produces an HTML fragment.
4. The host composes the full page from a template file and the widget fragments.
5. User interaction arrives via HTTP POST (`/instances/{id}/click/{plugin_id}`).
6. State updates are pushed via WebSocket as partial HTML replacements.

---

## 4. Why Headless Is Necessary

GTK rendering (`GtkSnapshot` + `GskRenderer`) requires an `ApplicationWindow` and a GDK surface. Without a window, there is no render context. Additionally, GTK
rendering must happen on the main thread.

For MacroPad devices (Stream Deck, Loupedeck):

- There is no display or window — the device is a USB-HID keyboard with LCD keys.
- Button images must be sent as **pixel buffers** to the device driver (e.g. `StreamDeck::set_button_image(key, DynamicImage)`).
- This happens asynchronously in the service thread, not on the GTK main thread.

The `GraphicRenderer` trait solves this: it renders with `image` + `ab_glyph` + `imageproc` (pure Rust, no GTK) and returns
`FfiGraphic { width, height, pixels }`.

---

## 5. How MacroPad Uses Headless Instances

```
service.macropad.connection (Connected)
  → route_message()
  → load_instance("macropad_1", "config-macropad-1.toml", InstanceType::Headless)
  → LauncherInstance created (no window)
  → device_metadata attached (key_count=15, key_width=72, key_height=72)
  → For each button: render_graphic(72, 72) → SetButtonImage sent to service

service.macropad.input (Button Press)
  → route_message()
  → instances.get("macropad_1")
  → Plugin.on_message() with simulated click
  → click_topic / click_payload / click_instance

service.macropad.connection (Disconnected)
  → route_message()
  → stop_instance("macropad_1")
  → Plugins unloaded, MCP tools deregistered, instance removed
```

---

## 6. MacroPadDeviceMetadata

The only additional data a headless MacroPad instance needs is device parameters for rendering:

```rust
/// Device-specific metadata for a MacroPad instance.
/// Attached to LauncherInstance after load_instance() completes.
/// None for GTK instances, Some(...) for headless MacroPad instances.
#[derive(Clone, Debug)]
pub struct MacroPadDeviceMetadata {
    /// Serial number or unique identifier of the device.
    pub device_id: String,
    /// Number of keys on the device (e.g. 15 for Stream Deck, 6+ for Loupedeck).
    pub key_count: u8,
    /// Key resolution width in pixels (e.g. 72 for Stream Deck, 90 for Loupedeck CT).
    pub key_width: u32,
    /// Key resolution height in pixels.
    pub key_height: u32,
    /// Driver/service that manages this device (e.g. "streamdeck", "loupedeck").
    pub driver: String,
}
```

This field is `Option<MacroPadDeviceMetadata>` on `LauncherInstance` — `None` for GTK instances, `Some(...)` for headless MacroPad instances. It is attached by
the `route_message()` handler after `load_instance()` completes, using data from the `MacroPadConnectionStatus` message.

---

## 7. References

- **Dynamic Load concept**: `concepts/DYNAMIC_LOAD_LAUNCHER_INSTANCE.md` — defines `InstanceType`, `load_instance()`, `stop_instance()`.
- **MacroPad concept**: `concepts/STREAMDECK_CONCEPT.md` — defines `GraphicRenderer` trait, `MacroPadDeviceMetadata`, and Headless instance pattern.
- **Web Instance concept**: `concepts/WEB_INSTANCE_CONCEPT.md` — defines `WebRenderer` trait, `WebInstanceMetadata`, template system, and HTTP server.
