# Concept: Configurable Icon Color for Widgets

This document defines the concept for adding a **user-configurable icon color** to the `WidgetIcon` struct. This allows users to specify a custom color for
widget icons via TOML configuration, complementing the existing semantic color system (`WidgetIconRendering` trait, `ViewData.icon_color`,
`AtomicGraphicData.icon_color`).

---

## 1. Problem Statement

### 1.1 Current State

The Swipe Launcher has two mechanisms for icon coloring:

1. **Semantic colors** (`WidgetIconRendering` trait in `plugin-api/src/widget/icon.rs`): Widgets return a `Color` based on their runtime state (e.g. green =
   safe, red = dangerous). This is used by weather widgets (temperature, UV index, air quality), precipitation levels, and similar data-driven widgets.

2. **Per-render colors** (`ViewData.icon_color` and `AtomicGraphicData.icon_color`): When constructing `ViewData` or `AtomicGraphicData`, widgets can attach an
   optional color that overrides the default text color for the icon. This is used by the centralised atomic rendering pipeline
   (`plugin-api/src/atomic/graphic.rs`) and by GTK widgets via CSS classes.

However, there is **no way for the user to configure a custom icon color** in the TOML configuration. All icon colors are either:

- Hardcoded defaults (white/text color)
- Determined by widget logic at runtime (semantic colors)

### 1.2 What Is Missing

- A `icon_color` field in the `WidgetIcon` struct that users can set in TOML.
- A parsing mechanism that converts a hex string (e.g. `"#ff6600"`) into the existing `Color` type.
- Integration with the GTK rendering path (CSS-based coloring).
- Integration with the headless rendering path (RGBA override in `AtomicGraphicData` / `ViewData`).
- A **priority model** that defines the precedence between configured, semantic, and default colors.

---

## 2. Goals

- Allow users to specify `icon_color` as a hex string in any widget config that uses `WidgetIcon` via `#[serde(flatten)]`.
- Support hex formats: `#rgb`, `#rrggbb`, `#rrggbbaa` (alpha optional, defaults to 255).
- The configured color acts as a **fallback** — semantic colors from `WidgetIconRendering` take priority.
- Works across all three rendering paths: GTK, Headless (GraphicRenderer), and Web (WebRenderer).
- No breaking changes to existing configs — `icon_color` is optional and defaults to `None`.

## 3. Non-Goals

- Replacing the semantic color system (`WidgetIconRendering`).
- Supporting CSS color names (e.g. `"red"`, `"blue"`) — hex strings only for simplicity and predictability.
- Adding per-view colors (each view can already set its own color via `ViewData`/`AtomicGraphicData`).
- Supporting gradients or patterns.

---

## 4. Architecture

### 4.1 Color Priority Model

When rendering an icon, the following priority order determines the effective color:

```
1. Error state          → error color (red/white, hardcoded)
2. Semantic color       → WidgetIconRendering::get_icon_color() (runtime)
3. Configured color     → WidgetIcon.icon_color (from TOML)
4. Default text color   → white or theme-dependent
```

This means:

- If a widget is in an error state, the error color is used (existing behaviour).
- If a widget returns a semantic color (e.g. temperature = blue), that color wins.
- If no semantic color is returned, but the user configured `icon_color`, that color is used.
- If neither is set, the default text color is used (existing behaviour).

### 4.2 TOML Configuration

The `icon_color` field is added to `WidgetIcon`, which is already flattened into widget configs via `#[serde(flatten)]`. Users specify it as a hex string:

```toml
[[main_menu.plugins]]
id = "my_weather"
path = "target/release/libsmearor_weather_widget.so"
widget = "weather"
icon_size = 36
icon_only = false
icon_color = "#4fc3f7"  # light blue
```

Supported formats:

| Format      | Example       | Description                              |
|-------------|---------------|------------------------------------------|
| `#rgb`      | `"#f60"`      | 4-bit per channel, expanded to `#ff6600` |
| `#rrggbb`   | `"#ff6600"`   | 8-bit per channel, alpha = 255           |
| `#rrggbbaa` | `"#ff660080"` | 8-bit per channel + alpha                |

### 4.3 Parsing

A `FromStr` implementation on `Color` parses hex strings. This follows the project convention of preferring trait implementations over free functions
(AGENTS.md: "Prefer Trait Implementations over Free Functions").

The existing `Color` struct (`plugin-api/src/widget/icon.rs`) needs an `a` (alpha) field added:

```rust
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,  // Alpha [0.0, 1.0], defaults to 1.0 (opaque)
}
```

All existing `Color` constants (e.g. `GREEN`, `RED`) set `a = 1.0`. The `to_rgba()` method already returns `[u8; 4]` — it uses `255` for alpha, which changes to
`(a * 255.0) as u8`.

```rust
impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Strip leading '#'
        // Match on remaining length: 3 (#rgb), 6 (#rrggbb), 8 (#rrggbbaa)
        // Parse hex digits, convert to f64 [0.0, 1.0]
        // For #rgb and #rrggbb: alpha defaults to 1.0
    }
}
```

The error type uses `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ColorParseError {
    #[error("invalid hex color: expected #rgb, #rrggbb, or #rrggbbaa, got '{0}'")]
    InvalidFormat(String),
    #[error("invalid hex digit in color '{0}'")]
    InvalidHexDigit(String),
}
```

### 4.4 WidgetIcon Extension

The `WidgetIcon` struct in `plugin-api/src/widget/icons.rs` gains an optional `icon_color` field. The field stores a pre-parsed `Color` (not a raw string), so
invalid hex values are caught at config load time with a clear error message including the TOML line number.

```rust
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetIcon {
    /// Icon size in pixels.
    #[builder(default, setter(into))]
    pub icon_size: i32,

    /// Show only the icon without text labels.
    #[builder(default, setter(into))]
    pub icon_only: bool,

    /// Optional icon color, parsed from a hex string (e.g. "#ff6600", "#f60", "#ff660080").
    /// Acts as a fallback when no semantic color is provided by the widget.
    #[serde(deserialize_with = "deserialize_hex_color", default)]
    #[builder(default, setter(into, strip_option))]
    pub icon_color: Option<Color>,
}
```

The custom serde deserializer `deserialize_hex_color` accepts a string in TOML and parses it via `Color::from_str()`. If parsing fails, serde returns an error
with the TOML location, providing early failure at config load time.

```rust
fn deserialize_hex_color<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) => s.parse::<Color>().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}
```

For serialisation, `Color` serialises back to a hex string via a custom `Serialize` impl (or `serialize_with`), ensuring round-trip consistency.

---

## 5. Rendering Integration

### 5.1 GTK Rendering

GTK widgets that use `WidgetIcon` via `#[serde(flatten)]` apply the configured color as a CSS override:

1. If `WidgetIconRendering::get_icon_color()` returns `Some(color)`, use the semantic color's CSS class (existing behaviour).
2. Else if `WidgetIcon.icon_color` is `Some(color)`, apply an inline CSS style or a custom CSS class with the color.
3. Else use the default icon color (existing behaviour).

Implementation approach: Add a `gtk4::CssProvider` to the icon `Image` widget's `StyleContext`. Since GTK CSS does not support arbitrary hex colors via class
names, an inline CSS override is needed.

**Important**: The CSS provider must be attached to the **widget's `StyleContext`**, not to the global `GdkDisplay`. Attaching to the display creates a new
provider on every render cycle, causing a memory leak and degrading GTK rendering performance.

```rust
if let Some(color) = icon_config.icon_color {
let css = format ! ("* {{ color: rgba({}, {}, {}, {}); }}",
(color.r * 255.0) as u8,
(color.g * 255.0) as u8,
(color.b * 255.0) as u8,
color.a);
let provider = gtk4::CssProvider::new();
provider.load_from_data( & css);
let style_context = icon_widget.style_context();
style_context.add_provider( & provider, gtk4::STYLE_PROVIDER_PRIORITY_USER);
}
```

GTK4 CSS supports `rgba()` natively, so the alpha channel is passed through without clamping.

The provider is scoped to the widget and automatically removed when the widget is destroyed. If the color changes, the widget is rebuilt from scratch (existing
behaviour for config changes), so no manual provider removal is needed.

### 5.2 Headless Rendering (GraphicRenderer / Atomic Widgets)

For atomic widgets, the centralised rendering pipeline in `plugin-api/src/atomic/graphic.rs` already accepts an `icon_color: Option<[u8; 4]>` parameter. The
widget's `render_atomic_graphic_data()` method constructs `AtomicGraphicData` and can pass the configured color as a fallback:

```rust
fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
    let mut data = AtomicGraphicData::new(/* ... */);

    // Semantic color takes priority
    if data.icon_color.is_none() {
        if let Some(color) = self.config.icon_config.icon_color {
            data.icon_color = Some(color.to_rgba());
        }
    }

    data
}
```

For multi-view widgets using `ViewData`, the same fallback logic applies:

```rust
fn build_view_data(&self) -> ViewData {
    let mut data = ViewData::new(/* ... */);

    if data.icon_color.is_none() {
        if let Some(color) = self.config.icon_config.icon_color {
            data.icon_color = Some(color);
        }
    }

    data
}
```

### 5.3 Web Rendering (WebRenderer)

Web widgets render HTML fragments. The configured color is applied as an inline CSS `color` style on the icon element:

```html
<i class="nf nf-weather-day_sunny" style="color: #ff6600;"></i>
```

The web rendering functions check `icon_config.icon_color` and, if set and no semantic color is present, inject the inline style.

---

## 6. Implementation Phases

### Phase 1: Core Infrastructure

| Task                          | File                             | Description                                              |
|-------------------------------|----------------------------------|----------------------------------------------------------|
| `ColorParseError` error type  | `plugin-api/src/widget/icon.rs`  | New error enum with `thiserror`                          |
| `FromStr` for `Color`         | `plugin-api/src/widget/icon.rs`  | Parse `#rgb`, `#rrggbb`, `#rrggbbaa`                     |
| `WidgetIcon.icon_color` field | `plugin-api/src/widget/icons.rs` | Add `Option<Color>` field with custom serde deserializer |
| `deserialize_hex_color`       | `plugin-api/src/widget/icons.rs` | Custom serde deserializer for hex string → `Color`       |
| Unit tests                    | `plugin-api/src/widget/icon.rs`  | Test valid/invalid hex parsing                           |

### Phase 2: GTK Integration

| Task                                  | File                                     | Description                                          |
|---------------------------------------|------------------------------------------|------------------------------------------------------|
| Apply configured color in GTK widgets | Each widget's `widget.rs` or `config.rs` | After semantic color check, apply CSS override       |
| Update `style.css`                    | `resources/style.css`                    | Ensure no conflicts with existing icon color classes |

### Phase 3: Headless Integration

| Task                                                | File                                                | Description                                    |
|-----------------------------------------------------|-----------------------------------------------------|------------------------------------------------|
| Pass configured color as fallback in atomic widgets | Each atomic widget's `render_atomic_graphic_data()` | Set `icon_color` when semantic color is `None` |
| Pass configured color in multi-view `ViewData`      | Each widget's `build_view_data()` or equivalent     | Set `icon_color` when semantic color is `None` |

### Phase 4: Web Integration

| Task                                      | File                          | Description                                |
|-------------------------------------------|-------------------------------|--------------------------------------------|
| Inject inline CSS color in HTML fragments | Each widget's `render_html()` | Add `style="color: #hex"` to icon elements |

### Phase 5: Documentation & Examples

| Task                   | File                      | Description                         |
|------------------------|---------------------------|-------------------------------------|
| Update example configs | `configs/launcher/*.toml` | Add commented `icon_color` examples |
| Update config docs     | `docs/` if applicable     | Document `icon_color` field         |

---

## 7. File Changes Summary

| File                                    | Change                                                                               |
|-----------------------------------------|--------------------------------------------------------------------------------------|
| `plugin-api/src/widget/icon.rs`         | Add `ColorParseError`, `FromStr` impl for `Color`, unit tests                        |
| `plugin-api/src/widget/icons.rs`        | Add `icon_color: Option<Color>` field, `deserialize_hex_color` serde helper          |
| `plugins/*/src/widget.rs`               | Apply configured color as fallback after semantic color check (GTK path)             |
| `plugins/*/src/atomic.rs`               | Apply configured color as fallback in `render_atomic_graphic_data()` (headless path) |
| `plugins/*/src/widget.rs` (WebRenderer) | Inject inline CSS color in `render_html()` output                                    |
| `resources/style.css`                   | Verify no conflicts with dynamic icon color overrides                                |
| `configs/launcher/*.toml`               | Add commented `icon_color` examples                                                  |

---

## 8. Dependencies

No new external dependencies. All required types (`Color`, `WidgetIcon`, `ViewData`, `AtomicGraphicData`) already exist in `plugin-api`. The `thiserror` crate
is already a workspace dependency.

---

## 9. Risks and Considerations

1. **Semantic vs. Configured Priority**: The configured color is a fallback, not an override. If a widget returns a semantic color (e.g. temperature = blue),
   the user's `icon_color` is ignored. This is intentional — semantic colors convey important information. Users who want to force a color can set
   `icon_only = true` and use a widget that does not implement `WidgetIconRendering`.

2. **Invalid Hex Strings**: Invalid hex strings cause a config load error with the TOML line number (early failure). This is preferable to silently falling back
   to the default color, as users get immediate feedback on misconfiguration.

3. **GTK CSS Specificity**: The CSS provider is attached to the widget's `StyleContext` (not the global `GdkDisplay`), preventing memory leaks from accumulating
   providers. The provider uses `STYLE_PROVIDER_PRIORITY_USER` and is scoped to the widget lifecycle — it is automatically removed when the widget is destroyed.
   The implementation should only apply the override when no semantic CSS class is active.

4. **Alpha Channel**: The `Color` struct gains an `a` (alpha) field. GTK4 CSS supports `rgba()` natively, so alpha is passed through in all rendering paths
   (GTK, headless, web). No clamping is applied.

5. **Performance**: The hex string is parsed once at config load time via the serde deserializer. No repeated parsing during rendering — the `Color`
   struct is stored directly in `WidgetIcon`.

6. **Web CSS Injection**: The hex string is parsed and validated before being injected into HTML. The `FromStr` implementation ensures only valid hex digits are
   accepted, preventing CSS injection attacks.

---

## 10. Resolved Questions

1. **Does `icon_color` support CSS color names** (e.g. `"red"`, `"blue"`)? — **No.** Hex strings only. They are unambiguous and locale-independent.

2. **Does the configured color apply to `main_text` and `info_text` as well?** — **No.** `icon_color` applies to the icon only. `text_color` will become a
   separate concept.

3. **Should there be a `background_color` for the icon area?** — **No.** This is covered by a separate concept:
   `concepts/planned/MACRO_PAD_ANIMATIONS_AND_BACKGROUND.md`.

4. **Should the parsed color be cached?** — **No.** Lazy parsing on every render call. The string is short and parsing is trivial.

---

## 11. References

- `plugin-api/src/widget/icon.rs` — `Color` struct, `WidgetIconRendering` trait
- `plugin-api/src/widget/icons.rs` — `WidgetIcon` struct
- `plugin-api/src/widget/view_data.rs` — `ViewData` with `icon_color` field
- `plugin-api/src/atomic/graphic_data.rs` — `AtomicGraphicData` with `icon_color` field
- `plugin-api/src/atomic/graphic.rs` — Centralised atomic rendering pipeline
- `concepts/inprogress/HEADLESS_WIDGETS_CONCEPT.md` — Headless rendering architecture
- `concepts/done/MACROPAD_ATOMIC_WIDGETS.md` — Atomic widget concept
- `AGENTS.md` — Project conventions (trait implementations, documentation standards)
