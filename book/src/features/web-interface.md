# Web Interface

The launcher supports web instances that serve their UI via HTTP and WebSocket, allowing remote control from a browser.

## Architecture

```mermaid
graph TB
    subgraph Host["LauncherHost"]
        WebServer["Web Server (axum)"]
        WS["WebSocket Connections"]
    end

    subgraph Instance["Web Instance"]
        Plugins["Plugins (WebRenderer)"]
        Areas["AreaManager"]
    end

    subgraph Browser["Browser Client"]
        Page["HTML Page"]
        JS["JavaScript (app.js)"]
    end

    Plugins --> WebServer: HTML fragments
    WebServer --> Browser: HTTP GET (full page)
    WS --> Browser: Partial HTML updates
    Browser --> WebServer: HTTP POST (click events)
    WebServer --> Plugins: Simulated click
```

## How It Works

1. **Page Load** — The browser requests the instance page. The host composes an HTML page from a template + widget fragments rendered by
   `WebRenderer::render_html()`.
2. **User Interaction** — Clicks are sent as HTTP POST requests to `/instances/{id}/click/{plugin_id}`. The host simulates a click on the plugin.
3. **State Updates** — When a widget's state changes, the host pushes a partial HTML replacement via WebSocket. The browser replaces the widget's DOM element.

## Web Renderer

Widget plugins implement `WebRenderer::render_html(instance_id, plugin_id)` to produce an HTML fragment:

```rust
pub trait WebRenderer {
    fn render_html(&self, instance_id: &str, plugin_id: &str) -> FfiHtmlString;
}
```

The host wraps fragments in a page template that includes CSS (`style.css`, `nerdfont.css`) and JavaScript (`app.js`).

## Configuration

Web instances are configured with `instance_type = "web"`:

```toml
[instances.web1]
instance_type = "web"
config_path = "configs/launcher/web.toml"
```

## Shared Infrastructure

Web instances share the same plugin, area, and messaging infrastructure as GTK and headless instances. The only difference is the rendering backend and input
method.

See [Instance Types](../architecture/instance-types.md) and [Renderer Systems](../architecture/renderer-systems.md) for technical details.
