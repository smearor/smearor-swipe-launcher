# Concept: Headless Widget Support — Multi-View Widgets on MacroPad & Web

All widget plugins must be usable in **headless** (`InstanceType::Headless`) and **web** (`InstanceType::Web`) instances — not just GTK instances. Currently
only the Button Widget implements `GraphicRenderer` (for MacroPad pixel buffers), and no widget implements `WebRenderer` (for web HTML fragments). Many widgets
have **multiple views** (e.g. popovers, expandable sections, sub-menus) that are trivial in GTK but have no headless or web equivalent.

This concept defines a unified rendering strategy, a view abstraction for multi-view widgets, and an implementation roadmap to bring headless and web support to
all widgets.

---

## 1. Problem Statement

### 1.1 Current State

| Widget               | `WidgetBuilder` (GTK) | `GraphicRenderer` (Headless) | `WebRenderer` (Web) | Multi-View?                          |
|----------------------|-----------------------|------------------------------|---------------------|--------------------------------------|
| `button`             | Yes                   | Yes                          | No                  | No (single button)                   |
| `app-launcher`       | Yes                   | No                           | No                  | No (single icon)                     |
| `audio`              | Yes                   | No                           | No                  | Yes (volume slider + popover)        |
| `clock`              | Yes                   | No                           | No                  | No                                   |
| `mpris`              | Yes                   | No                           | No                  | Yes (playback controls + seek bar)   |
| `network`            | Yes                   | No                           | No                  | Yes (status + dropdown menu)         |
| `notifications`      | Yes                   | No                           | No                  | Yes (popover with notification list) |
| `power`              | Yes                   | No                           | No                  | Yes (confirm/reveal sub-menu)        |
| `sysinfo` (×7)       | Yes                   | No                           | No                  | No (single metric each)              |
| `voice_assistant`    | Yes                   | No                           | No                  | Yes (status + conversation overlay)  |
| `wallpaper`          | Yes                   | No                           | No                  | Yes (thumbnail + selection grid)     |
| `weather`            | Yes                   | No                           | No                  | Yes (current + forecast detail)      |
| `workspace-switcher` | Yes                   | No                           | No                  | Yes (workspace grid)                 |

Only 1 of 19 widget implementations supports headless rendering. Zero support web rendering.

### 1.2 The Multi-View Challenge

GTK widgets use `Popover`, `Revealer`, `Dialog`, and nested containers to show/hide sub-views. In headless mode, a MacroPad button has a **fixed pixel grid**
(e.g. 72×72 px) — there is no popover. In web mode, HTML fragments can represent richer layouts, but the host needs to know which view is active to compose the
page.

The core problem: **how does a widget communicate its current view state and render that view on a non-GTK surface?**

---

## 2. Goals

- **Every widget** can render in headless (pixel buffer) and web (HTML fragment) mode.
- **Multi-view widgets** can switch between views in headless and web mode, just as they do in GTK.
- The host and area manager remain **view-agnostic** — they call `render_graphic()` or `render_html()` and receive the current view's output.
- No GTK dependency in `GraphicRenderer` or `WebRenderer` implementations.
- The `AreaBackend` abstraction for headless instances continues to work with no-op widget types.
- Web instances support partial updates (WebSocket) when a widget changes its view.

## 3. Non-Goals

- Changing the existing `WidgetBuilder` trait or GTK rendering path.
- Changing the `PluginVTable` structure (already has `render_graphic` and `render_html`).
- Replacing the `AreaBackend` trait or `HeadlessBackend` / `HeadlessWidget` no-op types.
- Adding new instance types beyond `Gtk`, `Headless`, and `Web`.
- Implementing touch/scroll support for MacroPad devices (hardware limitation).

---

## 4. Architecture

### 4.1 View Model

Each widget that has multiple views maintains an **active view** in its internal state. The view is an enum private to the widget:

```rust
/// Example: Audio widget views
enum AudioView {
    /// Compact: volume icon + percentage
    Compact,
    /// Expanded: volume slider + device selector + mute toggle
    Expanded,
}
```

The widget stores `current_view: AudioView` and updates it via message handling (e.g. `InvokeToolMessage` with `action: "toggle_view"` or `action: "expand"`).

### 4.2 Rendering by Instance Type

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Widget Plugin (.so)                            │
│                                                                      │
│  ┌─────────────┐  ┌──────────────────┐  ┌──────────────────────────┐ │
│  │ WidgetBuilder│  │ GraphicRenderer  │  │ WebRenderer              │ │
│  │ (GTK)        │  │ (Headless)       │  │ (Web)                    │ │
│  │              │  │                  │  │                          │ │
│  │ build_widget │  │ render_graphic   │  │ render_html              │ │
│  │  → gtk4::    │  │  → FfiGraphic    │  │  → String (HTML)         │ │
│  │    Widget    │  │  (RGBA pixels)   │  │                          │ │
│  └─────────────┘  └──────────────────┘  └──────────────────────────┘ │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    Internal State                                │ │
│  │  current_view: WidgetView  (enum, private to widget)            │ │
│  │  config: WidgetConfig                                          │ │
│  │  internal_state: ...                                           │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  All three renderers read from the same internal state.              │
│  The current_view determines what is rendered.                       │
└──────────────────────────────────────────────────────────────────────┘
```

### 4.3 View Switching

View switches are triggered by:

1. **`InvokeToolMessage`** — The widget registers an MCP tool (e.g. `audio_<plugin_id>`) with actions like `"expand"`, `"collapse"`, `"toggle_view"`. The host
   dispatches these when a MacroPad button is long-pressed or a web button is clicked.
2. **`state_topic` updates** — The widget receives a state update from its service that changes the view (e.g. `notifications` widget switches to `Expanded`
   when a new notification arrives).
3. **Internal timers** — The widget auto-collapses after a timeout (e.g. `power` widget returns to `Compact` after 5 seconds).

### 4.4 Headless Rendering for Multi-View Widgets

For MacroPad devices, each view produces a different pixel buffer:

```
AudioView::Compact          AudioView::Expanded
┌──────────────┐           ┌──────────────┐
│              │           │  ████░░░░ 60% │
│   🔊 60%     │    →      │              │
│              │           │  [Device]     │
│              │           │  [Mute]       │
└──────────────┘           └──────────────┘
   72 × 72 px                  72 × 72 px
```

The widget's `render_graphic(w, h)` matches on `current_view` and renders the appropriate layout. For small displays (72×72 px), expanded views use compact
representations:

- **Sliders**: Horizontal bar with fill percentage.
- **Lists**: First 3–4 items, scroll indicator if more.
- **Sub-menus**: Icon grid (2×2 or 3×3) within the button area.
- **Text**: Truncated to fit, with ellipsis.

### 4.5 Web Rendering for Multi-View Widgets

For web instances, each view produces a different HTML fragment:

```html
<!-- AudioView::Compact -->
<button class="smearor-button smearor-audio"
        data-plugin-id="audio_main"
        data-click-topic="tool.invoke"
        data-click-payload='{"tool":"audio_main","action":"expand"}'>
  <span class="smearor-button-icon nf-md-volume-high"></span>
  <span class="smearor-button-label">60%</span>
</button>

<!-- AudioView::Expanded -->
<div class="smearor-widget smearor-audio smearor-audio--expanded"
     data-plugin-id="audio_main">
  <div class="smearor-audio-slider">
    <input type="range" min="0" max="100" value="60"
           data-action="set_volume"
           data-plugin-id="audio_main" />
  </div>
  <div class="smearor-audio-devices">
    <button data-action="cycle_device" data-plugin-id="audio_main">Speakers</button>
  </div>
  <button class="smearor-audio-mute"
          data-action="toggle_mute"
          data-plugin-id="audio_main">Mute</button>
  <button class="smearor-audio-collapse"
          data-action="collapse"
          data-plugin-id="audio_main">▼</button>
</div>
```

The web client sends view-switch actions via HTTP POST to `/instances/{id}/click/{plugin_id}`. The host converts these to `InvokeToolMessage` and routes them to
the plugin. The plugin updates `current_view`, re-renders, and the host pushes the new HTML fragment via WebSocket.

### 4.6 Interaction Flow: MacroPad

```
1. User presses MacroPad button (short press)
   → MacroPadInputMessage { button_index, pressed: true }
   → Host: InvokeToolMessage { action: "click" }
   → Widget: executes click action (e.g. toggle mute)

2. User presses MacroPad button (long press)
   → MacroPadInputMessage { button_index, pressed: false } (after 500ms)
   → Host: InvokeToolMessage { action: "longpress" }
   → Widget: switches current_view (e.g. Compact → Expanded)
   → Widget: re-renders via render_graphic()
   → Host: sends SetButtonImage to MacroPad service
   → Device shows expanded view

3. User presses MacroPad button again (short press)
   → Host: InvokeToolMessage { action: "click" }
   → Widget: interprets click in context of current_view (Expanded)
   → Widget: e.g. cycles volume, selects device, etc.
```

### 4.7 Interaction Flow: Web

```
1. User clicks "Expand" button in HTML fragment
   → POST /instances/web_1/click/audio_main
   → Body: { "action": "expand" }
   → Host: InvokeToolMessage { action: "expand" }
   → Widget: switches current_view to Expanded
   → Widget: calls render_html() → new HTML fragment
   → Host: WebSocket push { "type": "update", "plugin_id": "audio_main", "html": "..." }
   → Browser: replaces DOM element
```

---

## 5. Widget View Catalogue

For each multi-view widget, the following views and rendering strategies are defined:

### 5.1 Audio Widget

| View       | Description              | Headless Rendering                 | Web Rendering                                                     |
|------------|--------------------------|------------------------------------|-------------------------------------------------------------------|
| `Compact`  | Volume icon + percentage | Icon + "60%" label                 | `<button>` with icon + label                                      |
| `Expanded` | Slider + device + mute   | Bar fill + device name + mute icon | `<div>` with `<input type="range">` + device button + mute button |

**View switch**: `longpress` (MacroPad) / `expand` action (Web).

### 5.2 MPRIS Widget

| View       | Description                  | Headless Rendering                 | Web Rendering                                                  |
|------------|------------------------------|------------------------------------|----------------------------------------------------------------|
| `Compact`  | Play/pause icon + track name | Icon + truncated title             | `<button>` with play/pause + title                             |
| `Expanded` | Controls + seek + metadata   | Play/pause + progress bar + artist | `<div>` with controls + `<input type="range">` seek + metadata |

**View switch**: `longpress` (MacroPad) / `expand` action (Web).

### 5.3 Network Widget

| View       | Description                       | Headless Rendering                   | Web Rendering                            |
|------------|-----------------------------------|--------------------------------------|------------------------------------------|
| `Compact`  | WiFi icon + SSID                  | Icon + SSID label                    | `<button>` with WiFi icon + SSID         |
| `Expanded` | Network list + connect/disconnect | 3–4 SSIDs as mini-rows + signal bars | `<div>` with SSID list + connect buttons |

**View switch**: `longpress` (MacroPad) / `expand` action (Web).

### 5.4 Notifications Widget

| View       | Description       | Headless Rendering                | Web Rendering                             |
|------------|-------------------|-----------------------------------|-------------------------------------------|
| `Compact`  | Bell icon + count | Icon + "3" label                  | `<button>` with bell icon + count badge   |
| `Expanded` | Notification list | First 2 notifications (truncated) | `<div>` with scrollable notification list |

**View switch**: `longpress` (MacroPad) / `expand` action (Web) / new notification arrival.

### 5.5 Power Widget

| View      | Description                             | Headless Rendering                                 | Web Rendering                 |
|-----------|-----------------------------------------|----------------------------------------------------|-------------------------------|
| `Compact` | Power icon                              | Icon only                                          | `<button>` with power icon    |
| `Confirm` | Shutdown / Reboot / Suspend / Hibernate | 2×2 icon grid (power, restart, suspend, hibernate) | `<div>` with 4 action buttons |

**View switch**: `click` (MacroPad) / `expand` action (Web). Auto-collapse after 10 seconds.

### 5.6 Voice Assistant Widget

| View        | Description                  | Headless Rendering            | Web Rendering                              |
|-------------|------------------------------|-------------------------------|--------------------------------------------|
| `Idle`      | Microphone icon              | Icon only                     | `<button>` with mic icon                   |
| `Listening` | Pulsing mic icon + waveform  | Mic icon + "..." label        | `<button>` with animated mic + status text |
| `Speaking`  | Speaker icon + response text | Speaker icon + truncated text | `<div>` with response text                 |

**View switch**: Driven by voice assistant service state (`state_topic`).

### 5.7 Wallpaper Widget

| View      | Description                  | Headless Rendering                 | Web Rendering                           |
|-----------|------------------------------|------------------------------------|-----------------------------------------|
| `Compact` | Current wallpaper thumbnail  | Scaled-down thumbnail image        | `<button>` with thumbnail as background |
| `Grid`    | 3×3 wallpaper selection grid | 3×3 thumbnail grid (24×24 px each) | `<div>` with CSS grid of thumbnails     |

**View switch**: `longpress` (MacroPad) / `expand` action (Web).

### 5.8 Weather Widget

| View       | Description                | Headless Rendering              | Web Rendering                      |
|------------|----------------------------|---------------------------------|------------------------------------|
| `Compact`  | Weather icon + temperature | Icon + "22°C" label             | `<button>` with icon + temperature |
| `Forecast` | 3-day forecast             | 3 mini rows (day + icon + temp) | `<div>` with 3-day forecast cards  |

**View switch**: `longpress` (MacroPad) / `expand` action (Web).

### 5.9 Workspace Switcher Widget

| View      | Description                          | Headless Rendering            | Web Rendering                    |
|-----------|--------------------------------------|-------------------------------|----------------------------------|
| `Compact` | Current workspace number + app count | "WS3" label + app count       | `<button>` with workspace number |
| `Grid`    | All workspaces with app indicators   | 2×3 grid of workspace numbers | `<div>` with workspace grid      |

**View switch**: `longpress` (MacroPad) / `expand` action (Web).

### 5.10 Single-View Widgets

The following widgets have only a `Compact` view and do not require view switching:

- **App Launcher**: Single icon + label. Click launches the app.
- **Button**: Single button with icon + label. Already supports `GraphicRenderer`.
- **Clock**: Time display. No interaction.
- **Sysinfo widgets** (CPU, Memory, Battery, Disks, Network, Uptime, Temperature): Single metric display. No interaction needed (read-only display).

For these widgets, `render_graphic()` and `render_html()` produce a single, static output based on current state.

---

## 6. Headless Rendering Strategies

### 6.1 Pixel Buffer Constraints

MacroPad devices have small displays (72×72 px for Stream Deck, 90×90 px for Loupedeck CT). Rendering must be compact:

| Element             | Strategy                                                                 |
|---------------------|--------------------------------------------------------------------------|
| **Icon**            | Centered, 40×40 px (Compact) or 20×20 px (Expanded sub-elements)         |
| **Label**           | Bottom-aligned, 10–12 px font, max 8 characters, truncated with ellipsis |
| **Slider**          | Horizontal bar, full width, 8 px tall, fill proportional to value        |
| **List**            | 2–3 rows, 16 px each, icon + truncated text                              |
| **Grid**            | 2×2 or 3×3 cells, 20×20 px each, icon only                               |
| **Progress**        | Bottom bar, 4 px tall, fill proportional                                 |
| **State indicator** | Top-right corner, 8×8 px colored dot                                     |

### 6.2 Font Rendering

All headless rendering uses the existing font infrastructure from `plugins/button/src/graphic.rs`:

- **Icons**: `SymbolsNerdFont-Regular.ttf` (NerdFont symbols-only).
- **Labels**: `JetBrainsMonoNLNerdFont-Regular.woff2` (full alphanumeric character set, decompressed via `woff2-patched`).

These fonts are cached in `OnceLock` and shared across all widgets. To avoid code duplication, the font loading and text rendering utilities should be extracted
into a **shared rendering crate** (see Section 8).

### 6.3 Image Rendering

For widgets that need to render images (e.g. Wallpaper thumbnail, App Launcher icon), the `image` crate is used to load and resize images to fit the target
dimensions. The `imageproc` crate provides drawing primitives (lines, rectangles, filled shapes).

### 6.4 Color Scheme

| Element               | Color     |
|-----------------------|-----------|
| Background (inactive) | `#1a1a2e` |
| Background (active)   | `#16213e` |
| Text (inactive)       | `#888888` |
| Text (active)         | `#e0e0e0` |
| Accent / highlight    | `#0f3460` |
| State indicator (on)  | `#00ff88` |
| State indicator (off) | `#444444` |

These match the existing Button Widget color scheme and should be shared via the rendering crate.

---

## 7. Web Rendering Strategies

### 7.1 HTML Fragment Structure

Each widget produces an HTML fragment with:

- **Root element**: `<button>` (compact) or `<div>` (expanded), with `data-plugin-id` attribute.
- **CSS classes**: `smearor-widget`, widget-specific class (e.g. `smearor-audio`), view class (e.g. `smearor-audio--expanded`).
- **Data attributes**: `data-click-topic`, `data-click-payload`, `data-action` for interaction wiring.
- **State classes**: `smearor-widget--active`, `smearor-widget--inactive` for state visualization.

### 7.2 CSS Framework

A shared CSS file (`resources/web/style.css`) provides base classes:

```css
.smearor-widget { /* base widget styles */
}

.smearor-widget--active { /* active state */
}

.smearor-widget--inactive { /* inactive state */
}

.smearor-widget--expanded { /* expanded view */
}

.smearor-button { /* compact button */
}

.smearor-button-icon { /* icon span */
}

.smearor-button-label { /* label span */
}

.smearor-slider { /* slider container */
}

.smearor-list { /* list container */
}

.smearor-grid { /* grid container */
}
```

Widget-specific styles are added per-widget (e.g. `.smearor-audio`, `.smearor-mpris`).

### 7.3 Interaction Wiring

Web interactions use the existing HTTP POST mechanism from `WEB_INSTANCE_CONCEPT.md`:

```javascript
// Click on compact view → expand
button.addEventListener('click', () => {
    fetch(`/instances/${instanceId}/click/${pluginId}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'expand' })
    });
});

// Slider input in expanded view → set volume
slider.addEventListener('input', (e) => {
    fetch(`/instances/${instanceId}/click/${pluginId}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'set_volume', value: e.target.value })
    });
});
```

### 7.4 WebSocket Updates

When a widget changes its view (or any state), the host:

1. Calls `render_html(instance_id, plugin_id)` on the widget.
2. Sends a WebSocket message: `{ "type": "update", "plugin_id": "...", "html": "..." }`.
3. The browser replaces the DOM element with matching `data-plugin-id`.

---

## 8. Shared Rendering Crate

### 8.1 Motivation

Currently, only `plugins/button` has headless rendering code. If every widget implements `GraphicRenderer` independently, font loading, color schemes, and
drawing utilities will be duplicated across 13+ plugin crates.

A shared crate `plugins/render-utils` (or `model/render-utils`) provides common rendering utilities:

### 8.2 Proposed API

```rust
/// Shared rendering utilities for headless widget rendering.
/// No GTK dependency — pure Rust with `image`, `ab_glyph`, `imageproc`.

/// Cached NerdFont (symbols-only) for icon rendering.
pub fn nerd_font() -> Option<&'static FontVec>;

/// Cached label font (JetBrains Mono Nerd Font) for text rendering.
pub fn label_font() -> Option<&'static FontVec>;

/// Draw a centered NerdFont icon on an RGBA image buffer.
pub fn draw_icon(image: &mut RgbaImage, icon_code: &str, size: u32, color: Rgba<u8>);

/// Draw a text label at the bottom of an image.
pub fn draw_label(image: &mut RgbaImage, text: &str, color: Rgba<u8>);

/// Draw a horizontal progress bar.
pub fn draw_progress_bar(image: &mut RgbaImage, value: f32, color: Rgba<u8>);

/// Draw a 2×2 or 3×3 icon grid.
pub fn draw_icon_grid(image: &mut RgbaImage, icons: &[&str], grid_cols: u32);

/// Fill the image background with a solid color.
pub fn fill_background(image: &mut RgbaImage, color: Rgba<u8>);

/// Default color scheme constants.
pub const COLOR_BACKGROUND: Rgba<u8>;
pub const COLOR_BACKGROUND_ACTIVE: Rgba<u8>;
pub const COLOR_TEXT: Rgba<u8>;
pub const COLOR_TEXT_ACTIVE: Rgba<u8>;
pub const COLOR_ACCENT: Rgba<u8>;
pub const COLOR_STATE_ON: Rgba<u8>;
pub const COLOR_STATE_OFF: Rgba<u8>;
```

### 8.3 Location

`plugins/render-utils/` as a workspace crate. All widget plugins add it as a dependency:

```toml
[dependencies]
smearor-render-utils = { path = "../render-utils" }
```

### 8.4 HTML Utilities

For web rendering, a simple set of HTML helper functions avoids string duplication:

```rust
/// Generate a compact button HTML fragment.
pub fn html_button(plugin_id: &str, icon_class: &str, label: &str, action: &str) -> String;

/// Generate an expanded view container opening tag.
pub fn html_expanded_open(plugin_id: &str, widget_class: &str) -> String;

/// Generate a slider input HTML element.
pub fn html_slider(plugin_id: &str, value: u32, action: &str) -> String;

/// Generate a list item HTML element.
pub fn html_list_item(icon_class: &str, text: &str, action: &str, plugin_id: &str) -> String;
```

---

## 9. MCP Tool Extensions

### 9.1 View Actions

Each multi-view widget registers MCP tool actions for view switching:

| Action        | Description                         | Behavior                                           |
|---------------|-------------------------------------|----------------------------------------------------|
| `expand`      | Switch to expanded view             | Sets `current_view = Expanded`, triggers re-render |
| `collapse`    | Switch to compact view              | Sets `current_view = Compact`, triggers re-render  |
| `toggle_view` | Toggle between compact and expanded | Flips `current_view`, triggers re-render           |

These are added to the widget's existing MCP tool schema alongside widget-specific actions.

### 9.2 Tool Schema Example (Audio)

```json
{
    "type": "object",
    "properties": {
        "action": {
            "type": "string",
            "enum": ["click", "longpress", "expand", "collapse", "toggle_view", "set_volume", "toggle_mute", "cycle_device"]
        },
        "value": {
            "type": "integer",
            "description": "Volume value (0-100) for set_volume action"
        }
    },
    "required": ["action"]
}
```

### 9.3 MacroPad Button Mapping

On MacroPad devices, the host maps button actions to view switches:

| Input       | Action        | Behavior                                       |
|-------------|---------------|------------------------------------------------|
| Short press | `click`       | Widget-specific action in current view context |
| Long press  | `toggle_view` | Switches between Compact and Expanded views    |

This is already implemented in `host/mod.rs` for the Button Widget. The same `InvokeToolMessage` mechanism routes `longpress` actions to any widget's
`handle_message()`.

---

## 10. Re-render Trigger

### 10.1 Headless Re-render

When a widget changes its view or receives a state update, the headless instance must re-render the button image. The current `render_buttons_to_device()`
function in `host/mod.rs` handles this for all visible plugins. The flow:

1. Widget receives `InvokeToolMessage` with `action: "expand"`.
2. Widget updates `current_view`.
3. Widget broadcasts a `WidgetUpdateMessage` on topic `widget.update` with its `plugin_id`.
4. Host receives `widget.update`, calls `render_graphic()` on the widget.
5. Host sends `SetButtonImage` to the MacroPad service.

### 10.2 Web Re-render

For web instances, the same `WidgetUpdateMessage` triggers:

1. Host calls `render_html()` on the widget.
2. Host sends WebSocket update to all connected browsers.

### 10.3 WidgetUpdateMessage

A new lightweight message type in `model/widget`:

```rust
/// Notification that a widget's visual state has changed and it needs re-rendering.
#[derive(Clone, Debug)]
#[stabby::stabby]
pub struct WidgetUpdateMessage {
    /// Plugin ID that needs re-rendering.
    pub plugin_id: stabby::string::String,
    /// Instance ID that owns the plugin.
    pub instance_id: stabby::string::String,
}
```

Topic: `widget.update`

This message is sent by any widget that changes its view or internal state in a way that affects rendering. The host listens for this topic and triggers the
appropriate re-render based on instance type.

---

## 11. Implementation Phases

### Phase 1: Shared Rendering Crate (`plugins/render-utils`)

**Order**: First. All widget phases depend on this.

**Changes**:

- Create `plugins/render-utils/` crate.
- Extract font loading (`nerd_font()`, `label_font()`) from `plugins/button/src/graphic.rs`.
- Extract drawing utilities (`draw_icon`, `draw_label`, `draw_progress_bar`, `fill_background`).
- Define color scheme constants.
- Add HTML helper functions (`html_button`, `html_expanded_open`, `html_slider`, `html_list_item`).
- Add to workspace `Cargo.toml`.
- Refactor `plugins/button/src/graphic.rs` to use `render-utils`.

**Exit Criteria**: Crate compiles, Button Widget still renders correctly using shared utilities.

### Phase 2: WidgetUpdateMessage (`model/widget`)

**Order**: After Phase 1.

**Changes**:

- Create `model/widget/` crate with `WidgetUpdateMessage` struct.
- Define topic `widget.update`.
- Add `register_json_converters()` for the message type.
- Add to workspace `Cargo.toml`.
- Host: add handler for `widget.update` topic in `route_message()` that triggers `render_buttons_to_device()` (headless) or WebSocket push (web).

**Exit Criteria**: Message type compiles, host receives and processes `widget.update` messages.

### Phase 3: Single-View Widgets — Headless + Web

**Order**: After Phase 2. Can be done in parallel per widget.

**Widgets**: `app-launcher`, `clock`, `sysinfo` (×7).

**Changes per widget**:

- Implement `GraphicRenderer::render_graphic()` using `render-utils`.
- Implement `WebRenderer::render_html()` using `render-utils` HTML helpers.
- Export `render_graphic` and `render_html` in the plugin's VTable.
- For `sysinfo`: implement for all 7 sub-widgets (CPU, Memory, Battery, Disks, Network, Uptime, Temperature).

**Exit Criteria**: Each widget renders correctly on MacroPad (72×72 px) and produces valid HTML fragments.

### Phase 4: Multi-View Widgets — Headless + Web

**Order**: After Phase 3. Can be done in parallel per widget.

**Widgets**: `audio`, `mpris`, `network`, `notifications`, `power`, `voice_assistant`, `wallpaper`, `weather`, `workspace-switcher`.

**Changes per widget**:

- Define `WidgetView` enum (private to widget).
- Add `current_view` field to widget struct.
- Implement view switching in `handle_message()` for `expand`, `collapse`, `toggle_view` actions.
- Add view actions to MCP tool schema.
- Implement `GraphicRenderer::render_graphic()` with view-aware rendering.
- Implement `WebRenderer::render_html()` with view-aware HTML fragments.
- Broadcast `WidgetUpdateMessage` on view change.
- Export `render_graphic` and `render_html` in the plugin's VTable.

**Exit Criteria**: Each widget can switch between Compact and Expanded views on MacroPad and Web. View switch is triggered by longpress (MacroPad) or button
click (Web).

### Phase 5: Web Server Integration

**Order**: After Phase 4. Depends on `WEB_INSTANCE_CONCEPT.md` Phase 3 (HTTP server).

**Changes**:

- Implement WebSocket push for `widget.update` messages.
- Client-side JavaScript: handle `update` messages, replace DOM elements.
- Implement view-switch click handling in `app.js`.
- Create CSS classes for all widget types and views.
- Test with a web instance containing all widgets.

**Exit Criteria**: All widgets render and respond to interactions in a web browser. View switches update the DOM via WebSocket without page reload.

### Phase 6: Polish and Testing

**Order**: After Phase 5.

**Changes**:

- Integration tests: load headless instance with all widgets, verify `render_graphic()` output.
- Integration tests: load web instance with all widgets, verify HTML fragments.
- Integration tests: verify view switching via `InvokeToolMessage`.
- Config examples: `config-macropad-all-widgets.toml`, `config-web-all-widgets.toml`.
- Documentation: README section for headless and web widget setup.
- Auto-collapse timers for `power`, `wallpaper`, `weather` expanded views (10 second timeout).
- Debouncing for rapid state updates.

**Exit Criteria**: All widgets work in GTK, Headless, and Web mode. View switching is responsive. No rendering artifacts on MacroPad devices.

---

## 12. File Changes Summary

| File                                       | Change                                                         |
|--------------------------------------------|----------------------------------------------------------------|
| `plugins/render-utils/Cargo.toml`          | **New** — shared rendering crate                               |
| `plugins/render-utils/src/lib.rs`          | **New** — font loading, drawing utilities, color constants     |
| `plugins/render-utils/src/html.rs`         | **New** — HTML helper functions                                |
| `model/widget/Cargo.toml`                  | **New** — widget update message model                          |
| `model/widget/src/lib.rs`                  | **New** — `WidgetUpdateMessage`, topic constant                |
| `plugins/button/src/graphic.rs`            | Refactor to use `render-utils`                                 |
| `plugins/app-launcher/src/widget.rs`       | Add `GraphicRenderer` + `WebRenderer` impl                     |
| `plugins/audio/src/widget.rs`              | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `plugins/clock/src/widget.rs`              | Add `GraphicRenderer` + `WebRenderer` impl                     |
| `plugins/mpris/src/widget.rs`              | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `plugins/network/src/widget.rs`            | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `plugins/notifications/src/widget.rs`      | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `plugins/power/src/widget.rs`              | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `plugins/sysinfo/src/widget_*.rs`          | Add `GraphicRenderer` + `WebRenderer` impl for each sub-widget |
| `plugins/voice_assistant/src/widget.rs`    | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `plugins/wallpaper/src/widget.rs`          | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `plugins/weather/src/widget.rs`            | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `plugins/workspace-switcher/src/widget.rs` | Add view enum, `GraphicRenderer` + `WebRenderer` impl          |
| `smearor-swipe-launcher/src/host/mod.rs`   | Add `widget.update` topic handler for re-rendering             |
| `resources/web/style.css`                  | Add widget-specific CSS classes                                |
| `resources/web/app.js`                     | Add view-switch click handling, WebSocket update handling      |
| `Cargo.toml` (workspace)                   | Add `smearor-render-utils`, `smearor-model-widget`             |

---

## 13. Dependencies

### New Crates

| Crate                  | Purpose                                                               |
|------------------------|-----------------------------------------------------------------------|
| `plugins/render-utils` | Shared font loading, drawing utilities, color constants, HTML helpers |
| `model/widget`         | `WidgetUpdateMessage` for re-render notifications                     |

### Per-Crate Additions

| Crate                    | Additional Dependencies                                              |
|--------------------------|----------------------------------------------------------------------|
| `plugins/render-utils`   | `image`, `ab_glyph`, `imageproc`, `woff2-patched`                    |
| `model/widget`           | `serde`, `serde_json`, `stabby`, `smearor-swipe-launcher-plugin-api` |
| All widget plugins       | `smearor-render-utils` (for `GraphicRenderer` / `WebRenderer` impls) |
| `smearor-swipe-launcher` | `smearor-model-widget` (for `widget.update` topic handler)           |

No new external dependencies — all required crates (`image`, `ab_glyph`, `imageproc`, `woff2-patched`) are already in the workspace.

---

## 14. Risks and Considerations

1. **Pixel buffer size**: 72×72 px is very limited for expanded views. Rendering must be carefully designed to fit content. Some widgets may need simplified
   expanded views on MacroPad (fewer items, smaller text).

2. **Font rendering performance**: Font rasterization with `ab_glyph` is CPU-bound. The `OnceLock` caching strategy ensures fonts are loaded once. Rendering is
   only triggered on state/view changes, not continuously.

3. **Web fragment size**: Expanded views produce larger HTML fragments. WebSocket updates should be debounced if a widget rapidly changes state (e.g. seek bar
   during playback).

4. **VTable compatibility**: The `PluginVTable` already has `render_graphic` (v2) and `render_html` (v3) as optional function pointers. Widgets that do not
   implement them set these to `None`. No VTable version bump is needed.

5. **Shared crate location**: `plugins/render-utils` is a library crate, not a plugin. It does not export `smearor_plugin_create`. It is consumed by other
   plugin crates as a dependency. Alternatively, it could be placed in `model/render-utils` to keep `plugins/` for actual plugins only.

6. **View state persistence**: View state (`current_view`) is stored in the widget struct and persists across re-renders. It is reset to `Compact` when the
   plugin is reloaded or the instance is restarted. Auto-collapse timers prevent widgets from being stuck in `Expanded` view.

7. **Cross-instance view state**: If a widget exists on both a GTK instance and a Headless instance, each has its own `current_view`. They are independent —
   expanding on MacroPad does not expand the GTK widget.

---

## 15. Open Questions

1. **render-utils crate location**: Should it be `plugins/render-utils` or `model/render-utils`? The `plugins/` directory is currently used for widget plugin
   crates that export `smearor_plugin_create`. A shared library crate might be better placed elsewhere.

2. **Expanded view on small MacroPad displays**: Is 72×72 px sufficient for expanded views, or should some widgets skip the expanded view on Stream Deck and
   only support it on Loupedeck CT (90×90 px) or Web?

3. **View state and area navigation**: When the user navigates to a different area and back, should the widget remember its view state, or reset to `Compact`?

4. **Web CSS framework**: Should the web instance use a CSS framework (e.g. TailwindCSS) or hand-written CSS? Hand-written CSS keeps the project dependency-free
   but requires more effort for responsive layouts.

5. **Slider input on web**: Should slider inputs send updates on every `input` event (real-time) or only on `change` (on release)? Real-time updates may cause
   excessive message traffic.

6. **Accessibility**: Should web fragments include ARIA attributes for accessibility (e.g. `aria-label`, `role="button"`)?

7. **Touch support on web**: Should the web instance support touch events for mobile devices (e.g. swipe to change area, long-press to expand)?

---

## 16. References

- **Instance Types**: `concepts/INSTANCE_TYPES.md` — compares Gtk, Headless, and Web instance types.
- **MacroPad concept**: `concepts/STREAMDECK_CONCEPT.md` — defines `GraphicRenderer` trait, `MacroPadDeviceMetadata`, and Headless instance pattern.
- **Web Instance concept**: `concepts/WEB_INSTANCE_CONCEPT.md` — defines `WebRenderer` trait, `WebInstanceMetadata`, template system, and HTTP server.
- **Widget Concept**: `concepts/WIDGET_CONCEPT.md` — general widget architecture and ABI stability.
- **Widget System**: `concepts/WIDGET_SYSTEM.md` — FFI communication and message broker.
- **Button Widget**: `concepts/BUTTON_WIDGET.md` — Button Widget configuration and implementation.
- **Multiple Widgets per Plugin**: `concepts/MULTIPLE_WIDGETS_PER_PLUGIN_CONCEPT.md` — `widget_factory_plugin!` macro for multi-widget crates.
- **Dynamic Load**: `concepts/DYNAMIC_LOAD_LAUNCHER_INSTANCE.md` — `InstanceType`, `load_instance()`, `stop_instance()`.
