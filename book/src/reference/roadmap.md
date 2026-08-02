# Roadmap

This page tracks the development progress of the Smearor Swipe Launcher.

## Phase 1: Foundation and Interface (MVP) — ✅ Complete

- [x] Project workspace setup
- [x] Plugin API design with state separation
- [x] C-ABI memory protection at FFI boundary (`stabby`)
- [x] First functional plugin (clock widget)
- [x] Dynamic plugin loading via `libloading`
- [x] Memory safety and allocator safety
- [x] First visual life sign (GTK 4 window with plugin widgets)

## Phase 2: Wayland Integration and Ribbon Layout — ✅ Complete

- [x] Three-part layout (left static, center scroll, right static)
- [x] Centralized gesture handling (click, long-press)

## Phase 3: Dynamic Configuration and Rotation — ✅ Complete

- [x] TOML configuration engine (`config.toml`)
- [x] Dynamic UI building from config
- [x] Parameter passing (JSON to plugins)
- [x] Rotation integration (`smearor-wrot-rotation`)
- [x] Rotation parameter propagation to app launches

## Phase 4: Core Widgets — 🔄 In Progress

- [ ] Non-blocking desktop entry parser (app-launcher)
- [ ] Two-way app execution with rotation
- [ ] MPRIS media plugin
- [ ] Time and calendar widget
- [ ] Notification widget (DBus listener, banner, badge)
- [ ] Layer-shell integration
- [ ] Exclusive zones
- [ ] Dynamic layer adjustment on rotation
- [ ] Virtualization of scroll ribbon (GtkListView/GridView)
- [ ] Touch optimization for 65" 4K smart desks

## Phase 5: Polishing and Performance — 🔄 In Progress

- [ ] Vertical swipes (up = parent menu, down = minimize)
- [ ] Keyboard navigation (SUPER + arrow keys)
- [ ] Hot-reloading CSS
- [ ] Performance fine-tuning of virtualization (120 Hz)
- [ ] Error encapsulation (panic handling for faulty plugins)

## Phase 6: MCP Server and AI Integration — ✅ Complete

- [x] MCP server crate (`rust-mcp-sdk` + `rust-mcp-axum`, Streamable HTTP + SSE)
- [x] Core tools (`open_area`, `close_area`, `list_areas`, `send_message`, etc.)
- [x] Plugin-tool-registry and plugin-resource-registry
- [x] Service plugin tools and resources (power, network, wallpaper, sysinfo, weather, audio, mpris, app-launcher)
- [x] MCP prompts (launcher_overview, area_control_help, broker_message_guide)
- [x] Plugin-prompt-registry with SSE notifications
- [x] Voice assistant integration (prompt catalog, context injection, system prompt extension)
- [x] Service plugin prompts (weather, mpris, terminal_command, app-launcher, power, voice_assistant)

## Beyond the Roadmap

Features implemented outside the original roadmap:

- **Multi-instance support** — Multiple launcher instances in a single host process
- **MacroPad integration** — Stream Deck and Loupedeck support via headless instances
- **Web instances** — Browser-based remote control via HTTP + WebSocket
- **Layout profiles** — Per-workspace area configuration
- **Action bindings** — Configurable user interaction mappings
- **Voice assistant** — Local LLM-based assistant with ReAct tool selection
- **Atomic widgets** — Multi-key MacroPad widget rendering with span groups
- **Debian packaging** — `cargo-deb` packaging with 33 packages (main + 13 widgets + 18 services + 1 metapackage), systemd user services, first-run config
  bootstrap, and dynamic dependency resolution via `$auto`
