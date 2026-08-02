# Plugin API Overview

The plugin API is defined in the `plugin-api` crate (`smearor-swipe-launcher-plugin-api`). It provides the traits, structs, and FFI machinery that plugins
implement to integrate with the launcher.

## What a Plugin Can Do

- **Render widgets** — GTK widgets (desktop), RGBA pixel buffers (headless), or HTML fragments (web)
- **Send and receive messages** — Via the central message broker using typed `FfiEnvelope` payloads
- **Register MCP tools** — Expose callable tools for AI clients and the voice assistant
- **Register MCP resources** — Expose readable data resources
- **Register MCP prompts** — Expose prompt templates for AI context injection
- **Spawn async tasks** — Via the host's tokio runtime (`PluginExecutor`)
- **Handle user input** — Click, long-press, double-press, hold, swipe, scroll, compound long-press
- **Configure action bindings** — Map user interactions to broker messages
- **Register JSON converters** — Convert generic JSON string payloads to typed messages

## Core Traits

### For Widget Plugins

| Trait                           | Purpose                                                         |
|---------------------------------|-----------------------------------------------------------------|
| `WidgetBuilder`                 | Build the GTK widget (`build_widget(rotation) -> gtk4::Widget`) |
| `GraphicRenderer`               | Render to RGBA pixels for headless instances                    |
| `WebRenderer`                   | Render to HTML for web instances                                |
| `MessageHandler<T>`             | Handle incoming typed messages                                  |
| `MessageBroadcaster`            | Send messages to the broker                                     |
| `PluginMetaGetter`              | Return plugin metadata (id, name, version)                      |
| `AsRef<Option<FfiCoreContext>>` | Provide access to the core context                              |

### For Service Plugins

| Trait                           | Purpose                            |
|---------------------------------|------------------------------------|
| `MessageHandler<T>`             | Handle incoming typed messages     |
| `MessageBroadcaster`            | Send messages to the broker        |
| `PluginMetaGetter`              | Return plugin metadata             |
| `AsRef<Option<FfiCoreContext>>` | Provide access to the core context |

## Macros

| Macro                          | Usage                                     |
|--------------------------------|-------------------------------------------|
| `widget_plugin!(MyWidget);`    | Export a widget plugin in `lib.rs`        |
| `service_plugin!(MyService);`  | Export a service plugin in `lib.rs`       |
| `impl_json_convertible!(...);` | Implement JSON conversion for model types |

## Crate Structure

A typical widget plugin crate:

```
plugins/my-widget/
├── Cargo.toml          # crate-type = ["cdylib"]
├── src/
│   ├── lib.rs          # widget_plugin!(MyWidget);
│   ├── widget.rs       # Widget struct + trait implementations
│   ├── config.rs       # Config struct with parse() method
│   └── atomic.rs       # Optional: atomic widget support
```

A typical service plugin crate:

```
services/my-service/
├── Cargo.toml          # crate-type = ["cdylib"]
├── src/
│   ├── lib.rs          # service_plugin!(MyService);
│   ├── service.rs      # Service struct + trait implementations
│   └── config.rs       # Config struct
```

A typical model crate:

```
model/my-model/
├── Cargo.toml          # stabby with serde feature
├── src/
│   └── lib.rs          # Structs, enums, topics, impl_json_convertible!, register_json_converters()
```

## Getting Started

- [Developing Widget Plugins](./widget-plugin.md) — Step-by-step guide
- [Developing Service Plugins](./service-plugin.md) — Step-by-step guide
- [Developing Model Crates](./model-crate.md) — Shared type definitions
- [Message System](./message-system.md) — How messaging works
- [Registering MCP Tools](./mcp-tools.md) — Expose tools to AI
- [Using Action Bindings](./action-bindings.md) — Configure user interactions
