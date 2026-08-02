# Renderer Systems

The launcher supports three rendering backends, determined by the instance type. All three share the same plugin and area infrastructure — they differ only in
how widgets are rendered to the user.

## Renderer Comparison

| Property          | GTK                             | Headless                            | Web                          |
|-------------------|---------------------------------|-------------------------------------|------------------------------|
| **Output**        | GTK widget tree                 | RGBA pixel buffer                   | HTML fragment                |
| **Trait**         | `WidgetBuilder::build_widget()` | `GraphicRenderer::render_graphic()` | `WebRenderer::render_html()` |
| **GTK required**  | Yes                             | No                                  | No                           |
| **Main thread**   | GLib main context               | No                                  | No                           |
| **User input**    | GTK events (click, touch)       | External (MacroPad buttons)         | HTTP POST / WebSocket        |
| **State updates** | GTK signal system               | Re-render + `SetButtonImage`        | WebSocket push               |
| **Use case**      | Desktop touch screen            | MacroPad devices                    | Remote / browser             |

## GTK Renderer

The default renderer for desktop instances. Widgets are built via `WidgetBuilder::build_widget()` and return a `gtk4::Widget` that is placed into the area
container. GTK handles all rendering, input events, and CSS styling.

## Graphic Renderer (Headless)

Used for MacroPad devices (Stream Deck, Loupedeck). Widgets implement `GraphicRenderer::render_graphic(width, height)` which returns an `FfiGraphic` struct
containing raw RGBA pixels. The rendering uses pure Rust crates (`image`, `ab_glyph`, `imageproc`) — no GTK dependency.

```rust
pub struct FfiGraphic {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA
}
```

The host renders each plugin's widget to a pixel buffer and sends it to the MacroPad service, which forwards it to the device driver.

## Web Renderer

Used for web instances. Widgets implement `WebRenderer::render_html(instance_id, plugin_id)` which returns an `FfiHtmlString` containing an HTML fragment. The
host composes the full page from a template and the fragments, serving it via an HTTP server.

State updates are pushed via WebSocket as partial HTML replacements.

```mermaid
graph LR
    subgraph Host["LauncherHost"]
        WebServer["Web Server (axum)"]
        WS["WebSocket"]
    end

    subgraph Instance["Web Instance"]
        Plugins["Plugins (WebRenderer)"]
    end

    subgraph Client["Browser"]
        Page["HTML Page"]
    end

    Plugins --> WebServer: HTML fragments
    WebServer --> Client: HTTP response
    WS --> Client: Partial updates
    Client --> WebServer: POST /click/{plugin_id}
    WebServer --> Plugins: Simulated click
```

## Atomic Widgets

For MacroPad devices, the launcher supports **atomic widgets** — widgets that render multiple buttons as a single combined image, then split it into individual
key images. This enables span groups (buttons spanning multiple keys) and 2D layouts.

See [Instance Types](./instance-types.md) for how instance types are chosen, and [MacroPad Integration](../features/macropad.md) for the feature perspective.
