# Concept: Atomic Widget Render Modes

This document defines the concept for **Render Modes** in Atomic Widgets — a generic mechanism that allows Atomic Widgets to draw custom graphics (backgrounds,
icons, or full-button graphics) instead of the default Nerd Font icon + text layout. This enables use cases such as MPRIS album art backgrounds, analog clocks,
CPU graphs, and dynamic app icons.

---

## 1. Problem Statement

### 1.1 Current State

Atomic Widgets render their MacroPad button graphics via a fixed pipeline:

1. `render_atomic_view(status, view)` returns `(icon_text, main_text, info_text)` — three strings.
2. `render_graphic()` in each widget's `atomic_graphic.rs` fills the background, draws the Nerd Font icon codepoint, then draws `main_text` and `info_text`.
3. Every Atomic Widget crate has its own `atomic_graphic.rs` duplicating this logic.

Limitations:

- **No custom graphics**: Icon is always a Nerd Font codepoint. Cannot draw album art, analog clocks, CPU graphs, or app icons.
- **No background images**: Background is always a solid colour.
- **No full-button rendering**: Widgets cannot take over the entire button area.
- **Code duplication**: Every crate has its own `atomic_graphic.rs`.
- **No text visibility control**: `main_text` and `info_text` are always drawn if non-empty.

### 1.2 What Is Missing

- **Render Modes**: Configure how an Atomic Widget renders its button.
- **Custom rendering hooks**: Optional functions for background, icon, or full-button graphics.
- **Text visibility control**: Configurable show/hide for `main_text` and `info_text`.
- **Centralised rendering pipeline**: Shared function replacing per-crate `atomic_graphic.rs`.

---

## 2. Goals

- Support **five render modes** covering all identified use cases.
- Provide **optional rendering hooks** (`render_background`, `render_icon_graphic`, `render_graphic`).
- Make render mode and text visibility **configurable** via `AtomicWidgetConfig`.
- Provide a **centralised rendering function** replacing per-crate `atomic_graphic.rs`.
- Maintain **backward compatibility**: widgets without custom hooks behave as today.
- Keep **GTK path unchanged**: render modes affect only headless pixel-buffer rendering.
- **`atomic_widget_impl!` macro** generates the `GraphicRenderer` impl automatically.

## 3. Non-Goals

- Changing the `GraphicRenderer` trait signature.
- Changing the `WidgetPluginVTable` structure.
- Supporting render modes in GTK instances.
- Supporting animated render modes (handled by the MacroPad Animation Engine concept).
- Changing the `render_atomic_view` return type.

---

## 4. Render Modes

| Mode               | Icon      | Background   | Text | Full Graphic | Use Case                                      |
|--------------------|-----------|--------------|------|--------------|-----------------------------------------------|
| **Icon**           | Nerd Font | Solid colour | Yes  | No           | Default — most widgets                        |
| **BackgroundOnly** | No        | Custom       | Yes  | No           | MPRIS Album — album art + text overlay        |
| **Background**     | Nerd Font | Custom       | Yes  | No           | Weather — icon + text over dynamic background |
| **GraphicOnly**    | No        | No           | No   | Yes          | Analog Clock, CPU graph — full custom graphic |
| **GraphicIcon**    | Custom    | Solid colour | Yes  | No           | App Launcher — custom app icon + text         |

### 4.1 Icon (Default)

Current behaviour. Nerd Font codepoint at top, `main_text` in middle, `info_text` at bottom.

```
┌────────┐
│  🔊   │  ← Nerd Font icon
│  60%   │  ← main_text
│ Vol    │  ← info_text
└────────┘
```

### 4.2 BackgroundOnly

Custom background fills the button (e.g. album art). No icon. Text overlaid with contrast.

```
┌────────┐
│ ░░░░░░ │  ← Custom background
│ ░Album░│  ← main_text
│ ░Name░ │  ← info_text
└────────┘
```

Widget implements `render_background()`. Fallback: solid colour if no background data.

### 4.3 Background

Custom background + Nerd Font icon + text.

```
┌────────┐
│ ░░☀️░░ │  ← Custom background + icon
│ ░22°C░ │  ← main_text
└────────┘
```

Widget implements `render_background()`. Icon and text drawn by centralised renderer.

### 4.4 GraphicOnly

Widget takes full control. No icon, no text, no solid background.

```
┌────────┐
│  ╭──╮  │
│  │  │  │  ← Full custom graphic
│  ╰──╯  │
└────────┘
```

Widget implements `render_graphic()` returning `true`. Fallback: Icon mode if returns `false`.

### 4.5 GraphicIcon

Custom icon graphic replaces Nerd Font codepoint. Text drawn as usual.

```
┌────────┐
│ [IMG]  │  ← Custom icon graphic
│ Firefox│  ← main_text
└────────┘
```

Widget implements `render_icon_graphic()`. Fallback: Nerd Font codepoint.

### 4.6 Fallback Summary

| Condition                                         | Fallback            |
|---------------------------------------------------|---------------------|
| Widget does not implement `AtomicGraphicRenderer` | Icon mode           |
| `render_graphic()` returns `false`                | Icon mode           |
| `render_background()` leaves buffer unchanged     | Solid colour        |
| `render_icon_graphic()` leaves buffer unchanged   | Nerd Font codepoint |
| `render_mode` not set in config                   | Icon mode           |

---

## 5. Architecture

### 5.1 AtomicGraphicRenderer Trait

New trait in `plugin-api/src/atomic/graphic.rs`. All methods have default no-op implementations.

```rust
/// Optional rendering hooks for Atomic Widget custom graphics.
///
/// Widgets implement this trait to provide custom background, icon, or
/// full-button graphics. All methods have default no-op implementations.
pub trait AtomicGraphicRenderer {
    /// Render a full-button graphic. Return `true` if rendered.
    /// Called only in `GraphicOnly` mode.
    fn render_graphic(&self, _pixels: &mut [u8], _width: u32, _height: u32) -> bool {
        false
    }

    /// Render a custom background filling the entire button.
    /// Return `true` if a background was drawn, `false` to use the solid colour fallback.
    /// Called in `BackgroundOnly` and `Background` modes.
    fn render_background(&self, _pixels: &mut [u8], _width: u32, _height: u32) -> bool {
        false
    }

    /// Render a custom icon graphic in the icon area.
    /// The icon area has the same dimensions as the Nerd Font icon: centered at
    /// `(width / 2, height * 0.35)` with size `(min(width, height) * 0.5).min(40.0)`.
    /// Called in `GraphicIcon` mode.
    fn render_icon_graphic(&self, _pixels: &mut [u8], _width: u32, _height: u32) -> bool {
        false
    }
}
```

### 5.2 AtomicRenderMode Enum

New enum in `plugin-api/src/atomic/config.rs`:

```rust
/// Render mode for an Atomic Widget's headless graphic output.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub enum AtomicRenderMode {
    /// Nerd Font icon + text on solid background (default).
    #[default]
    Icon,
    /// Custom background + text, no icon.
    BackgroundOnly,
    /// Custom background + Nerd Font icon + text.
    Background,
    /// Full custom graphic, no icon, no text.
    GraphicOnly,
    /// Custom icon graphic + text on solid background.
    GraphicIcon,
}
```

### 5.3 Config Extensions

`AtomicWidgetConfig` is extended:

```rust
pub struct AtomicWidgetConfig {
    // ... existing fields ...

    /// Render mode for headless graphic output. Defaults to `Icon`.
    pub render_mode: Option<AtomicRenderMode>,

    /// Whether to show `main_text` in headless rendering. Defaults to `true`.
    pub show_main_text: Option<bool>,

    /// Whether to show `info_text` in headless rendering. Defaults to `true`.
    pub show_info_text: Option<bool>,

    /// Opacity of the semi-transparent text backdrop (0.0 = transparent, 1.0 = opaque).
    /// Only used in `BackgroundOnly` and `Background` modes. Defaults to `0.5`.
    pub text_backdrop_opacity: Option<f32>,
}
```

### 5.4 Centralised Rendering Function

New function `render_atomic_graphic_default()` in `plugin-api/src/atomic/graphic.rs`:

```rust
pub fn render_atomic_graphic_default(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    icon_char: char,
    main_text: &str,
    info_text: &str,
    is_error: bool,
    config: &AtomicWidgetConfig,
    renderer: Option<&dyn AtomicGraphicRenderer>,
) {
    let mode = config.render_mode.as_ref().unwrap_or(&AtomicRenderMode::Icon);
    let is_graphic_only = *mode == AtomicRenderMode::GraphicOnly;
    let show_main = !is_graphic_only && config.show_main_text.unwrap_or(true);
    let show_info = !is_graphic_only && config.show_info_text.unwrap_or(true);

    // 1. GraphicOnly — widget takes over entirely
    if *mode == AtomicRenderMode::GraphicOnly {
        if let Some(r) = renderer {
            if r.render_graphic(pixels, width, height) {
                return;
            }
        }
        // Fallback: Icon mode
    }

    // 2. Background
    match mode {
        AtomicRenderMode::BackgroundOnly | AtomicRenderMode::Background => {
            let has_custom_bg = if let Some(r) = renderer {
                r.render_background(pixels, width, height)
            } else {
                false
            };
            if !has_custom_bg {
                fill_background(pixels, width, height, bg_color(is_error));
            }
        }
        _ => fill_background(pixels, width, height, bg_color(is_error)),
    }

    // 3. Icon
    match mode {
        AtomicRenderMode::Icon | AtomicRenderMode::Background => {
            draw_nerd_font_codepoint(pixels, width, height, icon_char, ...);
        }
        AtomicRenderMode::GraphicIcon => {
            let has_custom_icon = if let Some(r) = renderer {
                r.render_icon_graphic(pixels, width, height)
            } else {
                false
            };
            if !has_custom_icon {
                draw_nerd_font_codepoint(pixels, width, height, icon_char, ...);
            }
        }
        _ => {} // BackgroundOnly, GraphicOnly: no icon
    }

    // 4. Text (with semi-transparent backdrop for readability over custom backgrounds)
    let needs_backdrop = matches!(mode, AtomicRenderMode::BackgroundOnly | AtomicRenderMode::Background);
    let backdrop_opacity = config.text_backdrop_opacity.unwrap_or(0.5);
    if show_main && !main_text.is_empty() {
        if needs_backdrop {
            draw_text_backdrop(pixels, width, height, main_text, ..., backdrop_opacity);
        }
        draw_text_centered(pixels, width, height, main_text, ...);
    }
    if show_info && !info_text.is_empty() {
        if needs_backdrop {
            draw_text_backdrop(pixels, width, height, info_text, ..., backdrop_opacity);
        }
        draw_text_centered(pixels, width, height, info_text, ...);
    }
}
```

### 5.5 Macro Integration

The `atomic_widget_impl!` macro generates the `GraphicRenderer` impl automatically. The `renderer` parameter is passed as `Option<&dyn AtomicGraphicRenderer>`.
Since Rust does not allow conditional trait dispatch without `specialization` (unstable), the macro generates a blanket `AtomicGraphicRenderer` impl with no-op
defaults. Widgets that need custom rendering provide their own methods in a separate impl block — but since Rust does not allow split impls for the same trait,
the macro must **not** generate the `AtomicGraphicRenderer` impl.

**Solution**: The macro generates the `GraphicRenderer` impl that calls `render_atomic_graphic_default()` with `renderer: None`. Widgets that implement
`AtomicGraphicRenderer` override the `GraphicRenderer` impl by providing their own, calling `render_atomic_graphic_default()` with `Some(self)`.

**Better solution**: The macro accepts an optional `graphic_renderer: true` parameter. When set, it generates a `GraphicRenderer` impl that passes
`Some(self as &dyn AtomicGraphicRenderer)`. The widget must also provide an `AtomicGraphicRenderer` impl. When not set (default), the macro generates a
`GraphicRenderer` impl with `renderer: None` (Icon mode fallback).

```rust
atomic_widget_impl! {
    widget: MprisAtomicWidget,
    status: MprisStatusMessage,
    topic: TOPIC_STATUS,
    // ...
    graphic_renderer: true,  // ← opt-in custom rendering
}
```

### 5.6 Module Structure

```
plugin-api/src/atomic/
    mod.rs              — re-exports
    action.rs           — AtomicAction (unchanged)
    build.rs            — GTK widget construction (unchanged)
    config.rs           — AtomicWidgetConfig + AtomicRenderMode (extended)
    graphic.rs          — AtomicGraphicRenderer trait + render_atomic_graphic_default() (new)
    macro.rs            — atomic_widget_impl! macro (extended)
```

---

## 6. Rendering Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                  render_graphic(width, height)               │
│                   (generated by macro)                       │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Extract (icon_char, main_text, info_text, is_error)     │
│     from latest_status via render_atomic_view()              │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│  2. render_atomic_graphic_default()                          │
│     Centralised pipeline — dispatches based on render_mode   │
│                                                             │
│  ┌─ GraphicOnly? → render_graphic() → true? DONE            │
│  ├─ Background?  → render_background() or solid colour      │
│  ├─ Icon?        → draw_nerd_font_codepoint()                │
│  ├─ GraphicIcon? → render_icon_graphic() or fallback         │
│  └─ Text?        → draw_text_centered() (if show_main/info)  │
└─────────────────────────────────────────────────────────────┘
```

---

## 7. Configuration Examples

### 7.1 MPRIS Album (BackgroundOnly)

```toml
[mpris_album]
defaults = "menu_button"
path = "target/release/libsmearor_mpris_widget.so"
widget = "mpris_album"
render_mode = "background_only"
show_info_text = false
text_backdrop_opacity = 0.6
click_topic = "tool.invoke"
click_payload = { tool = "mpris", action = "play_pause" }
```

### 7.2 Analog Clock (GraphicOnly)

```toml
[clock_analog]
defaults = "menu_button"
path = "target/release/libsmearor_clock_widget.so"
widget = "clock_analog"
render_mode = "graphic_only"
show_main_text = false
show_info_text = false
```

### 7.3 App Launcher (GraphicIcon)

```toml
[firefox]
defaults = "menu_button"
path = "target/release/libsmearor_app_launcher_widget.so"
widget = "app_launcher"
render_mode = "graphic_icon"
click_topic = "tool.invoke"
click_payload = { tool = "app_launcher", action = "launch", app = "firefox" }
```

### 7.4 Weather with Background (Background)

```toml
[weather_today]
defaults = "menu_button"
path = "target/release/libsmearor_weather_widget.so"
widget = "weather_today"
render_mode = "background"
click_topic = "tool.invoke"
click_payload = { tool = "weather", action = "open_app" }
```

### 7.5 Default (Icon — no config needed)

```toml
[audio_volume]
defaults = "menu_button"
path = "target/release/libsmearor_audio_widget.so"
widget = "audio_volume"
click_topic = "tool.invoke"
click_payload = { tool = "audio", action = "volume_up" }
```

---

## 8. Widget Implementation Examples

### 8.1 MPRIS Album (BackgroundOnly)

```rust
// plugins/mpris/src/atomic_graphic.rs

impl AtomicGraphicRenderer for MprisAtomicWidget {
    fn render_background(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        let status = self.latest_status.borrow();
        if let Some(status) = status.as_ref() {
            if let Some(art) = &status.album_art {
                // Scale album art to button dimensions and copy into pixels
                draw_image_scaled(pixels, width, height, art);
                return true;
            }
        }
        // No album art → solid colour fallback
        false
    }
}
```

### 8.2 Analog Clock (GraphicOnly)

```rust
// plugins/clock/src/atomic_graphic.rs

impl AtomicGraphicRenderer for ClockAtomicWidget {
    fn render_graphic(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        let status = self.latest_status.borrow();
        let Some(status) = status.as_ref() else { return false; };

        // Draw analog clock face
        draw_clock_face(pixels, width, height, status.hour, status.minute);
        true
    }
}
```

### 8.3 App Launcher (GraphicIcon)

```rust
// plugins/app-launcher/src/atomic_graphic.rs

impl AtomicGraphicRenderer for AppLauncherAtomicWidget {
    fn render_icon_graphic(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        if let Some(icon) = &self.app_icon {
            // Draw app icon at the same size as a Nerd Font icon:
            // centered at (width / 2, height * 0.35), size = (min(width, height) * 0.5).min(40.0)
            let icon_size = (width.min(height) as f32 * 0.5).min(40.0);
            draw_icon_scaled(pixels, width, height, icon, icon_size);
            return true;
        }
        // No custom icon → Nerd Font fallback
        false
    }
}
```

### 8.4 Widget Without Custom Rendering (Icon — no change)

No `atomic_graphic.rs` file needed. The macro generates the `GraphicRenderer` impl with `renderer: None`, and the centralised function renders Icon mode.

---

## 9. Migration Path

The migration is organised into **four phases**. Each phase is independently buildable and testable — the project remains in a working state after each phase.

---

### Phase 1: Foundation — New Types and Trait (no behaviour change)

**Goal**: Add the new types, trait, and centralised rendering function to `plugin-api`. No existing widget is modified yet. All widgets continue to use their
per-crate `atomic_graphic.rs` files.

**Steps**:

1. **Add `AtomicRenderMode` enum** to `plugin-api/src/atomic/config.rs`:
    - Enum with variants: `Icon` (default), `BackgroundOnly`, `Background`, `GraphicOnly`, `GraphicIcon`.
    - Derive `Clone`, `Debug`, `Default`, `Deserialize`, `PartialEq`.
    - Add `render_mode: Option<AtomicRenderMode>`, `show_main_text: Option<bool>`, `show_info_text: Option<bool>` fields to `AtomicWidgetConfig`.

2. **Create `plugin-api/src/atomic/graphic.rs`**:
    - Define `AtomicGraphicRenderer` trait with `render_graphic()`, `render_background()`, `render_icon_graphic()` — all returning `bool`, all with default
      `false`/no-op implementations.
    - Implement `render_atomic_graphic_default()` — the centralised rendering pipeline that dispatches based on `AtomicRenderMode`.
    - Add `draw_text_backdrop()` helper for semi-transparent text backdrop (used in `BackgroundOnly`/`Background` modes).
    - Re-use existing functions from `smearor-render-utils`: `fill_background`, `draw_nerd_font_codepoint`, `draw_text_centered`, `background_color`,
      `text_color`.

3. **Update `plugin-api/src/atomic/mod.rs`**:
    - Add `mod graphic;`.
    - Re-export `AtomicGraphicRenderer`, `AtomicRenderMode`, `render_atomic_graphic_default`.

4. **Add `render-utils` dependency** to `plugin-api/Cargo.toml` (if not already present):
    - `smearor-render-utils = { path = "../plugins/render-utils" }`

**Verification**: `cargo build -p smearor-swipe-launcher-plugin-api` succeeds. No widget behaviour changes.

**Files changed**:

| File                               | Change                                           |
|------------------------------------|--------------------------------------------------|
| `plugin-api/src/atomic/config.rs`  | Add `AtomicRenderMode` enum, config fields       |
| `plugin-api/src/atomic/graphic.rs` | New file: trait + centralised rendering function |
| `plugin-api/src/atomic/mod.rs`     | Add module, re-exports                           |
| `plugin-api/Cargo.toml`            | Add `smearor-render-utils` dependency            |

---

### Phase 2: Macro Integration — Auto-generate GraphicRenderer (behaviour identical)

**Goal**: Extend the `atomic_widget_impl!` macro to generate a `GraphicRenderer` impl that calls `render_atomic_graphic_default()`. Add the
`graphic_renderer: true` opt-in parameter. Existing widgets are updated to use the macro-generated impl. Per-crate `atomic_graphic.rs` files are removed.

**Steps**:

1. **Extend `atomic_widget_impl!` macro** in `plugin-api/src/atomic/macro.rs`:
    - Add optional `graphic_renderer: $enabled:expr` parameter (default: `false`).
    - When `graphic_renderer: false` (or absent): generate `GraphicRenderer` impl that calls `render_atomic_graphic_default()` with `renderer: None`. This
      produces identical output to the current per-crate implementations.
    - When `graphic_renderer: true`: generate `GraphicRenderer` impl that calls `render_atomic_graphic_default()` with
      `renderer: Some(self as &dyn AtomicGraphicRenderer)`. The widget must provide an `AtomicGraphicRenderer` impl in a separate file.

2. **Extract `render_atomic_graphic_data()` helper**:
    - Each widget's `atomic_graphic.rs` currently has a private `render_atomic_graphic()` function that extracts `(icon_char, main_text, info_text, is_error)`
      from `latest_status` and `view`.
    - This function is widget-specific (different status types, views, default icons).
    - The macro calls a widget-provided function `render_atomic_graphic_data(&self) -> (char, String, String, bool)` that each widget implements in its
      `atomic.rs`.
    - Alternatively, the macro generates this function from the existing `$status`, `$view`, `$icon` parameters and a widget-provided `render_atomic_view()`
      function. This requires the macro to call `render_atomic_view()` and extract the first char from the icon string.

3. **Update `atomic_widget_impl!` macro** to generate the `render_atomic_graphic_data` call:
    - The macro already has access to `$status` (status type), `$icon` (default icon char), `$view` (view field).
    - Generated `GraphicRenderer::render_graphic()` body:
      ```rust
      fn render_graphic(&self, width: u32, height: u32) -> FfiGraphic {
          let mut pixels = vec![0u8; (width * height * 4) as usize];
          let status = self.latest_status.borrow();
          let (icon_char, main_text, info_text, is_error) = render_atomic_graphic_data(&status, &self.view);
          smearor_swipe_launcher_plugin_api::render_atomic_graphic_default(
              &mut pixels, width, height,
              icon_char, &main_text, &info_text, is_error,
              &self.config,
              <renderer>,
          );
          FfiGraphic::from_pixels(width, height, pixels)
      }
      ```
    - The `render_atomic_graphic_data()` function is generated by the macro from the `$status` type, `$view` field, `$icon` default, and the widget's existing
      `render_atomic_view()` function. It encapsulates the logic currently in each `atomic_graphic.rs`'s private `render_atomic_graphic()` function.

4. **Remove `plugins/audio/src/atomic_graphic.rs`**:
    - The `GraphicRenderer` impl is now macro-generated.
    - The `render_atomic_graphic()` helper logic moves into the macro-generated code.
    - Remove `pub(crate) mod atomic_graphic;` from `plugins/audio/src/lib.rs`.

5. **Remove `plugins/mpris/src/atomic_graphic.rs`**:
    - Same as audio.
    - Remove `pub(crate) mod atomic_graphic;` from `plugins/mpris/src/lib.rs`.

6. **Remove `plugins/weather/src/atomic_graphic.rs`**:
    - Same as audio.
    - Remove `pub(crate) mod atomic_graphic;` from `plugins/weather/src/lib.rs`.

7. **Update `atomic_widget_impl!` invocations** in `plugins/audio/src/atomic.rs`, `plugins/mpris/src/atomic.rs`, `plugins/weather/src/atomic.rs`:
    - No change needed for `graphic_renderer` parameter — all three use default `false` (Icon mode).
    - The macro now generates the `GraphicRenderer` impl automatically.

**Verification**: `cargo build` succeeds for all plugin crates. MacroPad rendering output is pixel-identical to before (same icon, same text, same positions).
Run `cargo test` for all plugin crates.

**Files changed**:

| File                                    | Change                                                                                               |
|-----------------------------------------|------------------------------------------------------------------------------------------------------|
| `plugin-api/src/atomic/macro.rs`        | Generate `GraphicRenderer` impl, add `graphic_renderer` param, generate `render_atomic_graphic_data` |
| `plugins/audio/src/atomic_graphic.rs`   | **Deleted**                                                                                          |
| `plugins/audio/src/lib.rs`              | Remove `mod atomic_graphic;`                                                                         |
| `plugins/mpris/src/atomic_graphic.rs`   | **Deleted**                                                                                          |
| `plugins/mpris/src/lib.rs`              | Remove `mod atomic_graphic;`                                                                         |
| `plugins/weather/src/atomic_graphic.rs` | **Deleted**                                                                                          |
| `plugins/weather/src/lib.rs`            | Remove `mod atomic_graphic;`                                                                         |

---

### Phase 3: Custom Rendering — Implement AtomicGraphicRenderer for specific widgets

**Goal**: Widgets that need custom graphics (MPRIS album art, analog clock, app launcher icon) implement `AtomicGraphicRenderer` and opt in via
`graphic_renderer: true`.

**Steps**:

1. **MPRIS Album (BackgroundOnly)**:
    - Add `graphic_renderer: true` to the `atomic_widget_impl!` invocation in `plugins/mpris/src/atomic.rs` (or a separate MPRIS Album widget variant).
    - Create `plugins/mpris/src/atomic_graphic.rs` (new file) with `impl AtomicGraphicRenderer for MprisAtomicWidget` implementing
      `render_background() -> bool`. This function only performs a fast `memcpy` from the pre-scaled buffer — no image scaling.
    - Add `album_art: Option<AlbumArt>` to `MprisStatusMessage` in `model/mpris`. `AlbumArt` holds pre-scaled RGBA pixel data at button dimensions (e.g. 72×72×4
      bytes).
    - **Pre-scale album art in the MPRIS service** (`services/mpris`): when a status update is received with new album art metadata, the service downloads and
      scales the image to the target button dimensions asynchronously via `tokio::spawn` before broadcasting the status message. This keeps the GLib Main Loop
      unblocked.
    - Add `draw_image_scaled()` function to `plugins/render-utils/src/drawing.rs` — used by the service for pre-scaling, not by `render_background()`.
    - Set `render_mode = "background_only"` in MPRIS Album widget config.

2. **Analog Clock (GraphicOnly)** — future widget:
    - Create `plugins/clock/` crate with `ClockAtomicWidget`.
    - Implement `AtomicGraphicRenderer::render_graphic() -> bool` drawing an analog clock face.
    - Add `draw_circle()`, `draw_line()`, `draw_hand()` helpers to `plugins/render-utils/src/drawing.rs`.
    - Set `render_mode = "graphic_only"`, `show_main_text = false`, `show_info_text = false` in config.

3. **App Launcher (GraphicIcon)** — future widget:
    - Add `graphic_renderer: true` to the `atomic_widget_impl!` invocation.
    - Implement `AtomicGraphicRenderer::render_icon_graphic() -> bool` drawing the app icon PNG.
    - Add `draw_icon_scaled()` function to `plugins/render-utils/src/drawing.rs` — draws an RGBA image at a given position and size.
    - Set `render_mode = "graphic_icon"` in config.

4. **SysInfo CPU Graph (GraphicOnly)** — future widget:
    - Implement `render_graphic() -> bool` drawing a CPU usage graph (line chart or bar chart).
    - Add `draw_line_chart()` function to `plugins/render-utils/src/drawing.rs`.
    - Set `render_mode = "graphic_only"`, `show_main_text = false`, `show_info_text = false`.

**Verification**: Each widget renders correctly on MacroPad. Test with `cargo test` and visual inspection on device or via pixel buffer dump.

**Files changed**:

| File                                  | Change                                           |
|---------------------------------------|--------------------------------------------------|
| `plugins/mpris/src/atomic.rs`         | Add `graphic_renderer: true` to macro invocation |
| `plugins/mpris/src/atomic_graphic.rs` | New file: `AtomicGraphicRenderer` impl for MPRIS |
| `plugins/mpris/src/lib.rs`            | Add `mod atomic_graphic;` back                   |
| `model/mpris/src/...`                 | Add `album_art` field to `MprisStatusMessage`    |
| `plugins/render-utils/src/drawing.rs` | Add `draw_image_scaled()`, `draw_icon_scaled()`  |
| `plugins/clock/`                      | New crate (future)                               |
| `plugins/render-utils/src/drawing.rs` | Add `draw_circle()`, `draw_line()` (future)      |

---

### Phase 4: Config Support — Parse render_mode from TOML

**Goal**: Ensure `AtomicWidgetConfig` correctly parses `render_mode`, `show_main_text`, `show_info_text` from TOML config files. Add validation and defaults.

**Steps**:

1. **Update `AtomicWidgetConfig::parse()`** in `plugin-api/src/atomic/config.rs`:
    - Parse `render_mode` as `Option<AtomicRenderMode>` from TOML string (e.g. `"background_only"` → `AtomicRenderMode::BackgroundOnly`).
    - Parse `show_main_text` and `show_info_text` as `Option<bool>`.
    - Validate that `render_mode = "graphic_only"` silently ignores `show_main_text` and `show_info_text` (hard-set to `false` internally). This makes the TOML
      config fault-tolerant — no warning or error, the text fields are simply not rendered.

2. **Add TOML deserialization** for `AtomicRenderMode`:
    - `Icon` ↔ `"icon"`
    - `BackgroundOnly` ↔ `"background_only"`
    - `Background` ↔ `"background"`
    - `GraphicOnly` ↔ `"graphic_only"`
    - `GraphicIcon` ↔ `"graphic_icon"`
    - Use `#[serde(rename_all = "snake_case")]` on the enum.

3. **Update config documentation**:
    - Document the three new config fields in the widget config format.
    - Add examples for each render mode.

4. **Add tests**:
    - Unit tests for `AtomicRenderMode` deserialization.
    - Unit test: default config → `render_mode = Icon`, `show_main_text = true`, `show_info_text = true`.
    - Unit test: `render_mode = "background_only"` → `AtomicRenderMode::BackgroundOnly`.

**Verification**: `cargo test -p smearor-swipe-launcher-plugin-api` passes. Config files with `render_mode` parse correctly.

**Files changed**:

| File                              | Change                                                  |
|-----------------------------------|---------------------------------------------------------|
| `plugin-api/src/atomic/config.rs` | Parse new fields, add `serde` rename, validation, tests |

---

### Phase Dependency Graph

```
Phase 1 (Foundation)
    │
    ▼
Phase 2 (Macro Integration)
    │
    ├──▶ Phase 3 (Custom Rendering) — can proceed in parallel per widget
    │
    └──▶ Phase 4 (Config Support) — can proceed in parallel with Phase 3
```

Phase 1 and Phase 2 are sequential. Phase 3 and Phase 4 can proceed in parallel after Phase 2, and Phase 3 itself can be done per-widget (MPRIS first, then
Clock, then App Launcher, etc.).

---

## 10. Resolved Design Decisions

- **Detecting whether `render_background()` actually drew something**: `render_background()` returns `bool`. `true` means a background was drawn; `false`
  triggers the solid colour fallback. The same pattern applies to `render_icon_graphic()`.
- **Text contrast over custom backgrounds**: The centralised renderer draws a semi-transparent backdrop behind `main_text` and `info_text` when the render mode
  is `BackgroundOnly` or `Background`. The backdrop opacity is configurable via `text_backdrop_opacity: Option<f32>` in `AtomicWidgetConfig` (default: `0.5`).
  This handles dynamic backgrounds with varying brightness — a higher opacity ensures readability over light album art, while a lower opacity preserves more of
  the background image. The backdrop colour is a dark neutral (e.g. `[0, 0, 0, opacity * 255]`) blended over the background pixels behind the text area.
- **Icon area dimensions for `GraphicIcon`**: The custom icon graphic uses the same dimensions as the Nerd Font icon: centered at `(width / 2, height * 0.35)`
  with size `(min(width, height) * 0.5).min(40.0)`. `render_icon_graphic()` receives the full pixel buffer and is expected to draw within these bounds.
- **Pixel buffer allocation strategy**: The `vec![0u8; (width * height * 4) as usize]` pattern in the macro-generated `render_graphic()` is not a performance
  concern. `render_graphic` is called only on state changes (area switch, widget update) — not per frame. At 15 buttons × ~20 KB (72×72×4), the allocation
  overhead is negligible. `FfiGraphic::from_pixels()` consumes the `Vec` via `into_boxed_slice()` + `mem::forget()`, so there is no double allocation — the
  buffer is transferred to FFI ownership and freed via `FfiGraphic::free()` (Drop impl). All existing widget `graphic.rs` files already use this pattern.
  Alternatives like `Vec::with_capacity` + `set_len` on uninitialised memory would require `unsafe` for negligible gain (one `memset` saved). A reusable buffer
  pool would require changing `FfiGraphic` from `Vec`-ownership to references, which is an FFI-breaking change. If frame-based animations (20 fps, see
  `MACRO_PAD_ANIMATIONS_AND_BACKGROUND.md`) are added in the future, a `FfiGraphicPool` or `ReusableBuffer` can be introduced as part of the Animation Engine
  concept — not this Render Mode concept.
- **Image scaling and threading**: `render_graphic()` is called synchronously on the GLib Main Context loop (`main_context.spawn_local` in `application.rs`).
  Synchronous image scaling (e.g. downscaling high-res album art from 500×500 to 72×72) would block the Main Loop and cause visible lag. Therefore, image
  scaling must **not** happen inside `render_background()` or any other `AtomicGraphicRenderer` hook. Instead, images are pre-scaled when received: the service
  (e.g. MPRIS service) scales the image to the target button dimensions asynchronously (in a `tokio::spawn` task) when the status update is received, and stores
  the pre-scaled RGBA pixel array in `MprisStatusMessage`. `render_background()` then only performs a fast `memcpy` from the pre-scaled buffer into the render
  buffer. This keeps the Main Loop unblocked.
