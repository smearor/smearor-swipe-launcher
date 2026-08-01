# Concept: Configurable Text Colors for Widgets

This document defines the concept for adding **user-configurable text colors** for `main_text` and `info_text` to widget configurations. This allows users to
specify custom colors for the two text lines independently, complementing the existing `icon_color` configuration (`concepts/planned/WIDGET_ICON_COLOR.md`).

---

## 1. Problem Statement

### 1.1 Current State

The Swipe Launcher uses the **Unified 4-Line Layout** across all widgets:

| Line | Height      | Content     | CSS Class          |
|------|-------------|-------------|--------------------|
| 0    | `icon_size` | Icon        | `nerd-icon`        |
| 1    | 20px        | `main_text` | `widget-main-text` |
| 2    | 16px        | `info_text` | `widget-info-text` |
| 3    | 16px        | spacer/bar  | —                  |

Text colors are currently **fixed** via CSS rules in `resources/style.css`:

```css
.widget-main-text {
    color: #ffffff;
}

.widget-info-text {
    color: #88ccff;
    opacity: 0.8;
}
```

There is no way for users to configure `main_text` or `info_text` colors in TOML. All text colors are either:

- Hardcoded CSS defaults (white for `main_text`, light blue for `info_text`)
- Derived from `text_color(is_active)` in headless pixel rendering (`render-utils/src/drawing.rs`)
- Derived from `text_color(false)` or `TEXT_COLOR_ERROR` in the atomic rendering pipeline (`plugin-api/src/atomic/graphic.rs`)

In contrast, `icon_color` is already configurable via `WidgetIcon.icon_color` (`concepts/planned/WIDGET_ICON_COLOR.md`) and supports a priority model:
semantic > configured > default.

### 1.2 What Is Missing

- A `main_text_color` and `info_text_color` field in widget configurations, parsed from hex strings.
- A `WidgetTextColors` struct (analogous to `WidgetIcon`) that can be flattened into widget configs via `#[serde(flatten)]`.
- `main_text_color` and `info_text_color` fields in `ViewData` for semantic/runtime text coloring.
- `main_text_color` and `info_text_color` fields in `AtomicGraphicData` for pixel-based rendering.
- A `apply_text_color()` GTK helper for `Label` widgets (analogous to `apply_icon_color()` for `Image`).
- Integration with all four rendering paths: GTK, Headless (GraphicRenderer), Atomic, and Web (WebRenderer).
- A **priority model** that defines the precedence between semantic, configured, and default text colors.

---

## 2. Goals

- Allow users to specify `main_text_color` and `info_text_color` as hex strings in any widget config that uses `WidgetTextColors` via `#[serde(flatten)]`.
- Support hex formats: `#rgb`, `#rrggbb`, `#rrggbbaa` (alpha optional, defaults to 255) — same formats as `icon_color`.
- The configured colors act as a **fallback** — semantic colors from `ViewData` take priority.
- Works across all four rendering paths: GTK, Headless (GraphicRenderer), Atomic, and Web (WebRenderer).
- No breaking changes to existing configs — both fields are optional and default to `None`.
- `main_text` and `info_text` colors are **independently configurable**.

## 3. Non-Goals

- Replacing the CSS-based default styling — CSS classes remain the fallback when no color is configured or provided semantically.
- Supporting CSS color names (e.g. `"red"`, `"blue"`) — hex strings only, consistent with `icon_color`.
- Supporting per-view text colors via separate config fields (each view can already set its own color via `ViewData`).
- Supporting gradients, patterns, or text shadows.
- Changing the font family or font size — those remain CSS-driven.

---

## 4. Architecture

### 4.1 Color Priority Model

When rendering `main_text` or `info_text`, the following priority order determines the effective color:

```
1. Error state          → error color (TEXT_COLOR_ERROR, hardcoded)
2. Semantic color       → ViewData.main_text_color / ViewData.info_text_color (runtime)
3. Configured color     → WidgetTextColors.main_text_color / info_text_color (from TOML)
4. Default text color   → CSS class (.widget-main-text / .widget-info-text) or text_color(false)
```

This means:

- If a widget is in an error state, the error color is used (existing behaviour).
- If a widget provides a semantic text color via `ViewData` (e.g. warning/critical coloring), that color wins.
- If no semantic color is provided, but the user configured `main_text_color` / `info_text_color`, that color is used.
- If neither is set, the default text color is used (CSS for GTK/Web, `text_color(false)` for pixel rendering).

This mirrors the `icon_color` priority model exactly (Section 4.1 of `WIDGET_ICON_COLOR.md`).

### 4.2 TOML Configuration

The `main_text_color` and `info_text_color` fields are added to a new `WidgetTextColors` struct, which is flattened into widget configs via
`#[serde(flatten)]` — the same pattern as `WidgetIcon`.

```toml
[[main_menu.plugins]]
id = "my_sysinfo"
path = "target/release/libsmearor_sysinfo_widget.so"
icon_size = 36
icon_only = false
icon_color = "#4fc3f7"       # light blue icon
main_text_color = "#ffffff"   # white main text
info_text_color = "#aaaaaa"   # grey info text
```

Supported formats (same as `icon_color`):

| Format      | Example       | Description                              |
|-------------|---------------|------------------------------------------|
| `#rgb`      | `"#f60"`      | 4-bit per channel, expanded to `#ff6600` |
| `#rrggbb`   | `"#ff6600"`   | 8-bit per channel, alpha = 255           |
| `#rrggbbaa` | `"#ff660080"` | 8-bit per channel + alpha                |

### 4.3 WidgetTextColors Struct

A new struct `WidgetTextColors` is added to `plugin-api/src/widget/text_colors.rs`, following the project convention of one struct per file (AGENTS.md: "One
struct/enum per file").

```rust
use crate::widget::Color;
use serde::Deserialize;
use serde::Serialize;
use typed_builder::TypedBuilder;

use crate::widget::icons::deserialize_hex_color;
use crate::widget::icons::serialize_hex_color;

/// Default main_text_color value.
pub const DEFAULT_MAIN_TEXT_COLOR: Option<Color> = None;

/// Default info_text_color value.
pub const DEFAULT_INFO_TEXT_COLOR: Option<Color> = None;

/// Widget text color configuration for GTK, atomic, headless, and web rendering.
///
/// When flattened into a widget config via `#[serde(flatten)]`, the TOML
/// fields `main_text_color` and `info_text_color` map directly to this struct.
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetTextColors {
    /// Optional color for the main text line, parsed from a hex string.
    /// Acts as a fallback when no semantic color is provided by the widget.
    #[serde(
        deserialize_with = "deserialize_hex_color",
        serialize_with = "serialize_hex_color",
        default
    )]
    #[builder(default, setter(into, strip_option))]
    pub main_text_color: Option<Color>,

    /// Optional color for the info text line, parsed from a hex string.
    /// Acts as a fallback when no semantic color is provided by the widget.
    #[serde(
        deserialize_with = "deserialize_hex_color",
        serialize_with = "serialize_hex_color",
        default
    )]
    #[builder(default, setter(into, strip_option))]
    pub info_text_color: Option<Color>,
}

impl Default for WidgetTextColors {
    fn default() -> Self {
        Self {
            main_text_color: DEFAULT_MAIN_TEXT_COLOR,
            info_text_color: DEFAULT_INFO_TEXT_COLOR,
        }
    }
}

impl WidgetTextColors {
    /// Returns the configured main text color, if set.
    pub fn main_text_color(&self) -> Option<Color> {
        self.main_text_color
    }

    /// Returns the configured info text color, if set.
    pub fn info_text_color(&self) -> Option<Color> {
        self.info_text_color
    }
}
```

The `deserialize_hex_color` and `serialize_hex_color` functions are reused from `plugin-api/src/widget/icons.rs` — they are already generic over `Option<Color>`
and not specific to `icon_color`. If they are currently private, they should be made `pub(crate)` or moved to a shared location in `plugin-api/src/widget/`.

### 4.4 ViewData Extension

The `ViewData` struct in `plugin-api/src/widget/view_data.rs` gains two optional text color fields:

```rust
#[derive(Debug, Clone, TypedBuilder)]
pub struct ViewData {
    #[builder(setter(into))]
    pub icon_name: String,
    #[builder(setter(into))]
    pub main_text: String,
    #[builder(setter(into))]
    pub info_text: String,
    pub icon_color: Option<Color>,
    /// Optional semantic color for the main text line.
    pub main_text_color: Option<Color>,
    /// Optional semantic color for the info text line.
    pub info_text_color: Option<Color>,
    pub is_error: bool,
}
```

All constructors must be updated:

- `new()`: sets both text colors to `None`
- `with_color()`: sets both text colors to `None` (existing constructor, backward compatible)
- `error()`: sets both text colors to `None`
- TypedBuilder: `#[builder(default, setter(into, strip_option))]` for both fields

A new constructor `with_text_colors()` is added for widgets that want to set semantic text colors:

```rust
pub fn with_text_colors(
    icon_name: String,
    main_text: String,
    info_text: String,
    icon_color: Option<Color>,
    main_text_color: Option<Color>,
    info_text_color: Option<Color>,
) -> Self {
    Self {
        icon_name,
        main_text,
        info_text,
        icon_color,
        main_text_color,
        info_text_color,
        is_error: false,
    }
}
```

### 4.5 AtomicGraphicData Extension

The `AtomicGraphicData` struct in `plugin-api/src/atomic/graphic_data.rs` gains two optional text color fields:

```rust
#[derive(Clone, Debug)]
pub struct AtomicGraphicData {
    pub icon_char: char,
    pub main_text: String,
    pub info_text: String,
    pub is_error: bool,
    pub icon_color: Option<[u8; 4]>,
    /// Optional RGBA color for the main text line.
    pub main_text_color: Option<[u8; 4]>,
    /// Optional RGBA color for the info text line.
    pub info_text_color: Option<[u8; 4]>,
}
```

All constructors (`new()`, `with_color()`, `error()`) must be updated to set both text colors to `None`.

A new constructor `with_text_colors()` is added:

```rust
pub fn with_text_colors(
    icon_char: char,
    main_text: String,
    info_text: String,
    icon_color: Option<[u8; 4]>,
    main_text_color: Option<[u8; 4]>,
    info_text_color: Option<[u8; 4]>,
) -> Self {
    Self {
        icon_char,
        main_text,
        info_text,
        is_error: false,
        icon_color,
        main_text_color,
        info_text_color,
    }
}
```

---

## 5. Rendering Integration

### 5.1 GTK Rendering

#### 5.1.1 `apply_text_color()` Helper

A new function `apply_text_color()` is added to `plugin-api/src/nerd_font.rs`, analogous to the existing `apply_icon_color()` function. It accepts
`Option<Color>` so it can be called unconditionally in `update_ui()` — when `None`, any previously applied `text-color-*` CSS class is removed and the label
falls back to its default CSS class (`.widget-main-text` / `.widget-info-text`).

```rust
/// Applies a configured text color to a GTK `Label` via a display-scoped `CssProvider`.
///
/// Accepts `Option<Color>` so callers can pass `None` to reset to the default CSS class.
/// On each call, all previously applied `text-color-*` CSS classes are removed from the
/// label before the new class (if any) is added. This prevents CSS provider accumulation
/// across repeated `update_ui()` calls (e.g. semantic color changes Normal → Warning → Critical).
pub fn apply_text_color(label: &gtk4::Label, color: Option<Color>) {
    // Remove all previously applied text-color-* CSS classes
    let existing_classes: Vec<String> = label
        .css_classes()
        .iter()
        .filter(|c| c.starts_with("text-color-"))
        .map(|c| c.to_string())
        .collect();

    for class_name in existing_classes {
        label.remove_css_class(&class_name);
    }

    if let Some(color) = color {
        let class_name = format!(
            "text-color-{:02x}{:02x}{:02x}{:02x}",
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8,
            (color.a * 255.0).round() as u8
        );
        label.add_css_class(&class_name);
        let css = format!(
            ".{} {{ color: rgba({}, {}, {}, {}); opacity: 1; }}",
            class_name,
            (color.r * 255.0).round() as u8,
            (color.g * 255.0).round() as u8,
            (color.b * 255.0).round() as u8,
            color.a
        );
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(&css);
        gtk4::style_context_add_provider_for_display(
            &label.display(),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }
}
```

The CSS class name uses the `text-color-` prefix (instead of `icon-color-`) to avoid collisions with the icon color CSS classes. The
`STYLE_PROVIDER_PRIORITY_USER` priority ensures the override takes precedence over the application-level CSS rules in `style.css`.

**Class Cleanup**: Before adding a new `text-color-*` class, the function removes all existing `text-color-*` classes from the label. This ensures that repeated
calls in `update_ui()` (e.g. semantic color transitions Normal → Warning → Critical) do not accumulate stale CSS classes on the widget. While the `CssProvider`
objects on the `GdkDisplay` are not explicitly removed (GTK4 does not provide a public API to remove display-scoped providers), the stale providers become
inert — their CSS rules target class names that no longer exist on any widget, so they have no effect. This is the same pattern used by `apply_icon_color()`.

**Opacity Override**: The dynamic CSS rule includes `opacity: 1;` to neutralise the `opacity: 0.8` from the `.widget-info-text` CSS class. Without this, the CSS
`opacity` property would compound with the `rgba()` alpha channel, producing an effective opacity of `alpha * 0.8` instead of the user's intended `alpha`. By
resetting `opacity` to `1`, the user's configured color is applied exactly as specified — transparency is controlled solely via the `rgba()` alpha channel. This
applies to both `main_text_color` and `info_text_color` for consistency, even though `.widget-main-text` does not set `opacity`.

#### 5.1.2 GTK Widget Integration

Widgets that use `WidgetTextColors` via `#[serde(flatten)]` apply the configured colors in `build_widget()`:

1. **`build_widget()`**: Call `apply_text_color(&value_label, config.text_colors.main_text_color())` and
   `apply_text_color(&info_label, config.text_colors.info_text_color())`. These can be called unconditionally — when `None`, the function is a no-op.
2. **`update_ui()`**: After updating label text, call `apply_text_color(label, view_data.main_text_color)` and
   `apply_text_color(label, view_data.info_text_color)`. Since `apply_text_color` accepts `Option<Color>`, this can be called unconditionally — when the
   ViewData field is `None`, the previously applied `text-color-*` class is removed and the label falls back to the configured color (re-applied from config) or
   the default CSS class. This mirrors the `update_icon_display()` pattern for icon colors.

Example for `SysinfoMultiWidget` (`plugins/sysinfo/src/multi_widget.rs`):

```rust
// In build_widget():
let value_label = Label::builder()
.label( if show_labels { "Loading..." } else { "" })
.css_classes(["widget-main-text"])
.build();
apply_text_color( & value_label, config.text_colors.main_text_color());
content_box.append( & value_label);

let info_label = Label::builder()
.label( if show_labels { "" } else { "" })
.css_classes(["widget-info-text"])
.build();
apply_text_color( & info_label, config.text_colors.info_text_color());
content_box.append( & info_label);
```

```rust
// In update_ui() and cycle_view():
if let Some( ref label) = * value_label.borrow() {
label.set_text( & view_data.main_text);
apply_text_color(label, view_data.main_text_color);
}
if let Some( ref label) = * info_label.borrow() {
label.set_text( & view_data.info_text);
apply_text_color(label, view_data.info_text_color);
}
```

#### 5.1.3 Atomic GTK Widgets

Atomic widgets use `build_atomic_widget()` from `plugin-api/src/atomic/build.rs`, which creates three labels (icon, main, info). The main and info labels
already have the CSS classes `widget-main-text` and `widget-info-text`.

The `update_ui()` method of each atomic widget must apply text colors after calling `update_labels()`:

```rust
fn update_ui(&self) {
    let view_data = self.view.render(/* ... */);
    update_labels(
        &*self.icon_label.borrow(),
        &*self.main_label.borrow(),
        &*self.info_label.borrow(),
        &icon_char.to_string(),
        &view_data.main_text,
        &view_data.info_text,
    );
    apply_atomic_text_color(&self.main_label, view_data.main_text_color);
    apply_atomic_text_color(&self.info_label, view_data.info_text_color);
}
```

A helper function `apply_atomic_text_color()` is added to each atomic widget crate (or to `plugin-api`), analogous to the existing `apply_atomic_icon_color()`:

```rust
fn apply_atomic_text_color(label: &Rc<RefCell<Option<Label>>>, color: Option<Color>) {
    if let Some(ref l) = *label.borrow() {
        apply_text_color(l, color);
    }
}
```

Since `apply_text_color()` accepts `Option<Color>` and handles class cleanup internally, `apply_atomic_text_color()` can be called unconditionally in
`update_ui()` — when `color` is `None`, any previously applied `text-color-*` class is removed and the label falls back to its default CSS class
(`.widget-main-text` / `.widget-info-text`).

### 5.2 Headless Rendering (GraphicRenderer)

#### 5.2.1 Sysinfo Direct GraphicRenderer Path

The `render_view_data_to_graphic()` function in `plugins/sysinfo/src/graphic.rs` currently uses a single `text_col` for both `main_text` and `info_text`:

```rust
// Current:
let text_col = if view_data.is_error { TEXT_COLOR_ERROR } else { text_color(false) };
draw_text_centered( & mut pixels, width, height, & view_data.main_text,..., text_col);
draw_text_centered( & mut pixels, width, height, & view_data.info_text,..., text_col);
```

This must be changed to resolve each text color independently:

```rust
// New:
let text_col = if view_data.is_error { TEXT_COLOR_ERROR } else { text_color(false) };
let main_text_col = view_data.main_text_color.map( | c| c.to_rgba()).unwrap_or(text_col);
let info_text_col = view_data.info_text_color.map( | c| c.to_rgba()).unwrap_or(text_col);

draw_text_centered( & mut pixels, width, height, & view_data.main_text,..., main_text_col);
draw_text_centered( & mut pixels, width, height, & view_data.info_text,..., info_text_col);
```

The `draw_text_centered()` function in `render-utils/src/drawing.rs` already accepts a `Color` parameter — no signature change needed.

#### 5.2.2 Other Direct GraphicRenderer Paths (button, app-launcher)

Widgets that use `draw_label_text()` (e.g. `plugins/button/src/graphic.rs`, `plugins/app-launcher/src/graphic.rs`) currently derive the color internally via
`text_color(is_active)`. The `draw_label_text()` function does **not** accept a color parameter.

To support configurable text colors on this path, `draw_label_text()` in `plugins/render-utils/src/drawing.rs` must be extended with an optional color override:

```rust
pub fn draw_label_text(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    text: &str,
    is_active: bool,
    text_color_override: Option<Color>,
) {
    // ...
    let color = text_color_override.unwrap_or_else(|| crate::colors::text_color(is_active));
    // ...
}
```

This is a **breaking signature change** for all callers of `draw_label_text()`. All call sites must be updated simultaneously. The `text_color_override`
parameter should be `Option<Color>` with `None` as the default to preserve existing behaviour for widgets that do not yet support configured colors.

Additionally, widgets that draw `info_text` via `draw_text_centered()` (e.g. `plugins/button/src/graphic.rs`) must pass the resolved info text color instead of
the default `text_color(is_active)`.

#### 5.2.3 Atomic Rendering Pipeline

The centralised `render_atomic_graphic_default()` function in `plugin-api/src/atomic/graphic.rs` currently uses a single `text_col` for both text lines:

```rust
// Current (line 85):
let text_col = if is_error { TEXT_COLOR_ERROR } else { text_color(false) };

// Lines 153, 168:
draw_text_centered(pixels, width, height, main_text,..., text_col);
draw_text_centered(pixels, width, height, info_text,..., text_col);
```

This must be extended with two new parameters and per-line color resolution:

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
    icon_color: Option<[u8; 4]>,
    main_text_color: Option<[u8; 4]>,   // NEW
    info_text_color: Option<[u8; 4]>,   // NEW
) {
    // ...
    let text_col = if is_error { TEXT_COLOR_ERROR } else { text_color(false) };
    let main_col = main_text_color.unwrap_or(text_col);
    let info_col = info_text_color.unwrap_or(text_col);

    // Line 153:
    draw_text_centered(pixels, width, height, main_text, ..., main_col);
    // Line 168:
    draw_text_centered(pixels, width, height, info_text, ..., info_col);
}
```

This is a **breaking signature change**. The `atomic_widget_impl!` macro in `plugin-api/src/atomic/macro.rs` must be updated to pass `data.main_text_color` and
`data.info_text_color` from the `AtomicGraphicData` to `render_atomic_graphic_default()`. All four `@body` arms that call `render_atomic_graphic_default()` must
be updated.

### 5.3 Web Rendering (WebRenderer)

#### 5.3.1 Sysinfo WebRenderer

The `render_view_data_to_html()` function in `plugins/sysinfo/src/html.rs` currently applies `color_style` only to the icon element. Text elements use CSS
classes without inline color styles:

```rust
// Current (lines 44, 46):
html.push_str( & format!(r#"<div class="smearor-{}-main">{}</div>"#, widget_class, view_data.main_text));
html.push_str( & format!(r#"<div class="smearor-{}-info{}">{}</div>"#, widget_class, marquee_class, view_data.info_text));
```

This must be extended to inject inline `style="color: rgba(...)"` when text colors are present:

```rust
let main_color_style = if let Some(color) = view_data.main_text_color {
format!(
    r#" style="color: rgba({}, {}, {}, {}); opacity: 1;""#,
    (color.r * 255.0).round() as u8,
    (color.g * 255.0).round() as u8,
    (color.b * 255.0).round() as u8,
    color.a
)
} else {
String::new()
};

let info_color_style = if let Some(color) = view_data.info_text_color {
format!(
    r#" style="color: rgba({}, {}, {}, {}); opacity: 1;""#,
    (color.r * 255.0).round() as u8,
    (color.g * 255.0).round() as u8,
    (color.b * 255.0).round() as u8,
    color.a
)
} else {
String::new()
};

html.push_str( & format!(r#"<div class="smearor-{}-main"{}>{}</div>"#, widget_class, main_color_style, view_data.main_text));
html.push_str( & format!(r#"<div class="smearor-{}-info{}"{}>{}</div>"#, widget_class, marquee_class, info_color_style, view_data.info_text));
```

This follows the exact same pattern as the existing `color_style` for `icon_color` (lines 29–39).

#### 5.3.2 Other WebRenderer Implementations

Widgets that render HTML directly (e.g. `plugins/button/src/html.rs`, `plugins/app-launcher/src/html.rs`) must apply the same pattern: inject
`style="color: rgba(...)"` on text elements when configured colors are present.

For widgets that use CSS classes like `widget-main-text` and `widget-info-text` (e.g. `plugins/app-launcher/src/html.rs`), the inline style overrides the CSS
class color via CSS specificity (inline styles have higher priority than class rules).

### 5.4 CSS (`resources/style.css`)

**No changes required.** The existing `.widget-main-text` and `.widget-info-text` CSS rules provide the default colors:

```css
.widget-main-text {
    color: #ffffff;
}

.widget-info-text {
    color: #88ccff;
    opacity: 0.8;
}
```

The `apply_text_color()` function overrides these via display-scoped `CssProvider` with `STYLE_PROVIDER_PRIORITY_USER`, which has higher priority than the
application-level CSS. For web rendering, inline `style` attributes have higher CSS specificity than class rules.

---

## 6. Config Integration

### 6.1 SysinfoMultiWidgetConfig

The `SysinfoMultiWidgetConfig` in `plugins/sysinfo/src/config.rs` gains a `WidgetTextColors` flatten:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SysinfoMultiWidgetConfig {
    #[serde(flatten)]
    pub dimensions: WidgetDimensions,
    #[serde(flatten)]
    pub layout: WidgetLayout,
    #[serde(flatten)]
    pub icon_config: WidgetIcon,
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,   // NEW
    pub mode: WidgetMode,
    pub views: Vec<SysinfoView>,
    pub actions: ActionBindings,
}
```

`Default` implementation: `text_colors: WidgetTextColors::default()`.

### 6.2 PercentageWidgetConfig

The `PercentageWidgetConfig` in `plugins/sysinfo/src/config.rs` gains `WidgetTextColors`:

```rust
pub struct PercentageWidgetConfig {
    // ... existing fields ...
    #[serde(flatten)]
    pub text_colors: WidgetTextColors,   // NEW
}
```

### 6.3 Other Widget Configs

All widget configs that currently use `WidgetIcon` via `#[serde(flatten)]` should also gain `WidgetTextColors`:

- `plugins/button/src/config.rs` — `ButtonWidgetConfig`
- `plugins/app-launcher/src/config.rs` — `AppLauncherWidgetConfig`
- `plugins/weather/src/config.rs` — `WeatherWidgetConfig`
- `plugins/audio/src/config.rs` — `AudioWidgetConfig`
- `plugins/mpris/src/config.rs` — `MprisWidgetConfig`
- `plugins/power/src/config.rs` — `PowerWidgetConfig`
- `plugins/network/src/config.rs` — `NetworkWidgetConfig`
- `plugins/wallpaper/src/config.rs` — `WallpaperWidgetConfig`
- `plugins/clock/src/config.rs` — `ClockWidgetConfig`
- `plugins/workspace-switcher/src/config.rs` — `WorkspaceSwitcherConfig`

For atomic widgets, `AtomicWidgetConfig` in `model/widget/` (or `plugin-api/src/atomic/config.rs`) should also gain `WidgetTextColors` if text color
configuration is desired for atomic widgets.

---

## 7. ViewData Propagation

### 7.1 Sysinfo `render_view()` Function

The `render_view()` function in `plugins/sysinfo/src/multi_widget.rs` constructs `ViewData` for each sysinfo view. It currently sets `icon_color` from semantic
levels (e.g. `UsageLevel::from_percent(usage).get_icon_color()`).

To support configurable text colors, the function must also set `main_text_color` and `info_text_color`. Since sysinfo views do not currently have semantic text
colors, the configured colors from `WidgetTextColors` are used as the ViewData values:

```rust
pub(crate) fn render_view(
    view: SysinfoView,
    cpu: &Option<CpuStatusMessage>,
    // ... other status params ...
    override_data: &PersonalizationOverride,
    text_colors: &WidgetTextColors,   // NEW parameter
) -> ViewData {
    match view {
        SysinfoView::Cpu => {
            // ... existing logic ...
            ViewData::with_color(icon, format!("{:.0}%", usage), label.to_string(), color)
                // Set text colors from config
                .with_main_text_color(text_colors.main_text_color())
                .with_info_text_color(text_colors.info_text_color())
        }
        // ... other views ...
    }
}
```

Alternatively, the text colors can be set after construction:

```rust
let mut view_data = ViewData::with_color(icon, main_text, info_text, icon_color);
view_data.main_text_color = text_colors.main_text_color();
view_data.info_text_color = text_colors.info_text_color();
```

**Future semantic text colors**: If sysinfo views later want to color text based on warning/critical levels (e.g. red text for high CPU), the `render_view()`
function can set `main_text_color` from the `UsageLevel` and override the configured color. This follows the same priority model as `icon_color`.

### 7.2 Sysinfo Atomic `render()` Function

The `SysinfoAtomicView::render()` method in `plugins/sysinfo/src/atomic.rs` constructs `ViewData` similarly. It must also propagate text colors from the config.

Since `SysinfoAtomicView::render()` does not have access to the widget config, the `render_atomic_graphic_data()` method (which does have access to
`self.config`) should set the text colors on the `AtomicGraphicData`:

```rust
fn render_atomic_graphic_data(&self) -> AtomicGraphicData {
    let view_data = self.view.render(/* ... */);
    let icon_char = resolve_icon_codepoint(&view_data.icon_name).unwrap_or('\u{f2db}');
    let mut data = AtomicGraphicData::new(icon_char, view_data.main_text, view_data.info_text);
    data.is_error = view_data.is_error;
    data.icon_color = view_data.icon_color.map(|c| c.to_rgba());
    data.main_text_color = view_data.main_text_color.map(|c| c.to_rgba());
    data.info_text_color = view_data.info_text_color.map(|c| c.to_rgba());
    data
}
```

### 7.3 Other Widgets

Each widget's `render_view()` or equivalent function must propagate text colors from config to `ViewData`. For widgets that do not have semantic text colors,
this is simply:

```rust
view_data.main_text_color = self .config.text_colors.main_text_color();
view_data.info_text_color = self .config.text_colors.info_text_color();
```

---

## 8. Implementation Phases

### Phase 1: Core Infrastructure

| Task                                  | File                                    | Description                                                    |
|---------------------------------------|-----------------------------------------|----------------------------------------------------------------|
| `WidgetTextColors` struct             | `plugin-api/src/widget/text_colors.rs`  | New struct with `main_text_color`, `info_text_color` fields    |
| Re-export `WidgetTextColors`          | `plugin-api/src/widget/mod.rs`          | Re-export from module                                          |
| Re-export `apply_text_color`          | `plugin-api/src/lib.rs`                 | Re-export `apply_text_color` from `nerd_font` module           |
| `ViewData` text color fields          | `plugin-api/src/widget/view_data.rs`    | Add `main_text_color`, `info_text_color` fields + constructors |
| `AtomicGraphicData` text color fields | `plugin-api/src/atomic/graphic_data.rs` | Add `main_text_color`, `info_text_color` fields + constructors |
| `apply_text_color()` helper           | `plugin-api/src/nerd_font.rs`           | New function for GTK `Label` color override                    |
| Unit tests                            | `plugin-api/src/widget/text_colors.rs`  | Test serde round-trip, default values                          |

### Phase 2: Atomic Pipeline Extension

| Task                                               | File                               | Description                                                            |
|----------------------------------------------------|------------------------------------|------------------------------------------------------------------------|
| Extend `render_atomic_graphic_default()` signature | `plugin-api/src/atomic/graphic.rs` | Add `main_text_color`, `info_text_color` parameters                    |
| Update `atomic_widget_impl!` macro                 | `plugin-api/src/atomic/macro.rs`   | Pass `data.main_text_color`, `data.info_text_color` to render function |
| Update all `render_atomic_graphic_data()` methods  | Each atomic widget's `atomic.rs`   | Set `data.main_text_color` / `info_text_color` from ViewData           |

### Phase 3: Sysinfo Widget Integration

| Task                                                 | File                                  | Description                                              |
|------------------------------------------------------|---------------------------------------|----------------------------------------------------------|
| Add `WidgetTextColors` to `SysinfoMultiWidgetConfig` | `plugins/sysinfo/src/config.rs`       | `#[serde(flatten)] text_colors: WidgetTextColors`        |
| Add `WidgetTextColors` to `PercentageWidgetConfig`   | `plugins/sysinfo/src/config.rs`       | `#[serde(flatten)] text_colors: WidgetTextColors`        |
| Apply colors in `build_widget()`                     | `plugins/sysinfo/src/multi_widget.rs` | Call `apply_text_color()` on labels in `build_widget()`  |
| Apply colors in `update_ui()` / `cycle_view()`       | `plugins/sysinfo/src/multi_widget.rs` | Apply semantic text colors from ViewData                 |
| Propagate colors in `render_view()`                  | `plugins/sysinfo/src/multi_widget.rs` | Set `main_text_color` / `info_text_color` on ViewData    |
| Apply colors in `render_view_data_to_graphic()`      | `plugins/sysinfo/src/graphic.rs`      | Use per-line color resolution                            |
| Apply colors in `render_view_data_to_html()`         | `plugins/sysinfo/src/html.rs`         | Inject inline `style` on text divs                       |
| Apply colors in atomic `update_ui()`                 | `plugins/sysinfo/src/atomic.rs`       | Call `apply_atomic_text_color()` after `update_labels()` |

### Phase 4: Other Widget Integration

| Task                                         | File                                  | Description                                        |
|----------------------------------------------|---------------------------------------|----------------------------------------------------|
| Add `WidgetTextColors` to all widget configs | Each widget's `config.rs`             | `#[serde(flatten)] text_colors: WidgetTextColors`  |
| Apply colors in `build_widget()`             | Each widget's `widget.rs`             | Call `apply_text_color()` on labels                |
| Apply colors in `update_ui()`                | Each widget's `widget.rs`             | Apply semantic text colors from ViewData           |
| Extend `draw_label_text()` signature         | `plugins/render-utils/src/drawing.rs` | Add `text_color_override: Option<Color>` parameter |
| Update all `draw_label_text()` callers       | Each widget's `graphic.rs`            | Pass resolved text color                           |
| Apply colors in `render_html()`              | Each widget's `html.rs`               | Inject inline `style` on text elements             |

### Phase 5: Documentation & Examples

| Task                                      | File                                    | Description                                                  |
|-------------------------------------------|-----------------------------------------|--------------------------------------------------------------|
| Update example configs                    | `configs/launcher/*.toml`               | Add commented `main_text_color` / `info_text_color` examples |
| Update `ICON_RENDERING.md`                | `docs/ICON_RENDERING.md`                | Document text color fields and priority model                |
| Update `WIDGET_ICON_COLOR.md` resolved Q2 | `concepts/planned/WIDGET_ICON_COLOR.md` | Update cross-reference to this concept                       |

---

## 9. File Changes Summary

| File                                    | Change                                                                            |
|-----------------------------------------|-----------------------------------------------------------------------------------|
| `plugin-api/src/widget/text_colors.rs`  | **New**: `WidgetTextColors` struct, defaults, accessor methods, unit tests        |
| `plugin-api/src/widget/mod.rs`          | Re-export `WidgetTextColors`                                                      |
| `plugin-api/src/widget/view_data.rs`    | Add `main_text_color`, `info_text_color` fields + constructors                    |
| `plugin-api/src/atomic/graphic_data.rs` | Add `main_text_color`, `info_text_color` fields + constructors                    |
| `plugin-api/src/atomic/graphic.rs`      | Extend `render_atomic_graphic_default()` with text color parameters               |
| `plugin-api/src/atomic/macro.rs`        | Pass text colors from `AtomicGraphicData` to render function                      |
| `plugin-api/src/nerd_font.rs`           | Add `apply_text_color()` for GTK `Label`                                          |
| `plugin-api/src/lib.rs`                 | Re-export `WidgetTextColors`, `apply_text_color`                                  |
| `plugins/sysinfo/src/config.rs`         | Add `WidgetTextColors` to `SysinfoMultiWidgetConfig` and `PercentageWidgetConfig` |
| `plugins/sysinfo/src/multi_widget.rs`   | Apply colors in `build_widget()`, `update_ui()`, `cycle_view()`, `render_view()`  |
| `plugins/sysinfo/src/graphic.rs`        | Per-line color resolution in `render_view_data_to_graphic()`                      |
| `plugins/sysinfo/src/html.rs`           | Inline color styles in `render_view_data_to_html()`                               |
| `plugins/sysinfo/src/atomic.rs`         | Apply text colors in `update_ui()`, `render_atomic_graphic_data()`                |
| `plugins/render-utils/src/drawing.rs`   | Extend `draw_label_text()` with `text_color_override` parameter                   |
| `plugins/*/src/config.rs`               | Add `WidgetTextColors` to all widget configs                                      |
| `plugins/*/src/widget.rs`               | Apply colors in `build_widget()` and `update_ui()` (GTK path)                     |
| `plugins/*/src/graphic.rs`              | Pass text colors to `draw_label_text()` / `draw_text_centered()` (headless path)  |
| `plugins/*/src/html.rs`                 | Inject inline color styles in `render_html()` (web path)                          |
| `plugins/*/src/atomic.rs`               | Apply text colors in `update_ui()`, `render_atomic_graphic_data()` (atomic path)  |
| `configs/launcher/*.toml`               | Add commented `main_text_color` / `info_text_color` examples                      |
| `docs/ICON_RENDERING.md`                | Document text color fields                                                        |

---

## 10. Dependencies

No new external dependencies. All required types (`Color`, `WidgetIcon`, `ViewData`, `AtomicGraphicData`) already exist in `plugin-api`. The
`deserialize_hex_color` and `serialize_hex_color` functions already exist in `plugin-api/src/widget/icons.rs` and are reused for `WidgetTextColors`.

---

## 11. Risks and Considerations

1. **Semantic vs. Configured Priority**: The configured text color is a fallback, not an override. If a widget provides a semantic text color via `ViewData`
   (e.g. red text for critical CPU usage), the user's `main_text_color` is ignored. This is intentional — semantic colors convey important information. This
   mirrors the `icon_color` priority model.

2. **GTK CSS Provider Accumulation**: The `apply_text_color()` function removes all existing `text-color-*` CSS classes from the label before adding a new one,
   preventing stale class accumulation on the widget across repeated `update_ui()` calls (e.g. semantic transitions Normal → Warning → Critical). While the
   `CssProvider` objects on the `GdkDisplay` are not explicitly removed (GTK4 does not provide a public API to remove display-scoped providers), stale providers
   become inert — their CSS rules target class names that no longer exist on any widget. This is the same pattern used by `apply_icon_color()`. The function
   accepts `Option<Color>`, so it can be called unconditionally in `update_ui()` with `None` to reset to the default CSS class.

3. **Atomic Pipeline Signature Change**: Extending `render_atomic_graphic_default()` with two new parameters is a breaking change for all callers. The
   `atomic_widget_impl!` macro generates all call sites, so the change is centralized — only the macro needs updating. However, any widget that calls
   `render_atomic_graphic_default()` directly (outside the macro) must also be updated.

4. **`draw_label_text()` Signature Change**: Extending `draw_label_text()` with a `text_color_override` parameter is a breaking change for all callers. All call
   sites must be updated simultaneously. The parameter defaults to `None` to preserve existing behaviour.

5. **Alpha Channel and CSS `opacity` Compounding**: The `Color` struct already supports alpha via the `a` field. GTK4 CSS supports `rgba()` natively, and the
   pixel rendering pipeline uses `[u8; 4]` RGBA. No clamping is applied. The `.widget-info-text` CSS class sets `opacity: 0.8`, which would compound with the
   `rgba()` alpha channel (effective opacity = `alpha * 0.8`). To prevent this, both the dynamic GTK CSS rule and the web inline style include `opacity: 1;`
   when a text color is configured. This ensures the user's configured color is applied exactly as specified — transparency is controlled solely via the
   `rgba()` alpha channel. See Resolved Question #3 for details.

6. **CSS Specificity (Web)**: Inline `style` attributes have higher CSS specificity than class rules. The existing `.widget-main-text` and `.widget-info-text`
   CSS rules are overridden by the inline `style="color: rgba(...)"` when text colors are configured. When no text color is configured, the CSS class rules
   apply as before.

7. **Performance**: Hex strings are parsed once at config load time via the serde deserializer. No repeated parsing during rendering — the `Color` struct is
   stored directly in `WidgetTextColors`. The `apply_text_color()` function creates a new `CssProvider` per call, but this is only triggered on widget
   construction and view updates, not on every frame.

8. **Web CSS Injection**: The hex string is parsed and validated before being injected into HTML. The `FromStr` implementation ensures only valid hex digits are
   accepted, preventing CSS injection attacks (same consideration as `icon_color`).

9. **`WidgetIcon` vs. `WidgetTextColors` Separation**: Text colors are in a separate struct (`WidgetTextColors`) rather than added to `WidgetIcon`. This keeps
   `WidgetIcon` focused on icon properties and allows widgets to opt into text color configuration independently of icon configuration. Both structs are
   flattened via `#[serde(flatten)]` into the same widget config, so the TOML fields are at the same level.

---

## 12. Resolved Questions

1. **Should `main_text_color` and `info_text_color` be in `WidgetIcon` or a separate struct?** — **Separate struct (`WidgetTextColors`).** `WidgetIcon` is
   focused on icon properties. Text colors are a separate concern. Both are flattened into widget configs via `#[serde(flatten)]`, so the TOML fields are at the
   same level. This follows the project convention of one struct per file.

2. **Should there be semantic text colors (e.g. warning/critical)?** — **Yes, via `ViewData`.** The `ViewData` struct gains `main_text_color` and
   `info_text_color` fields. Widgets can set these at runtime to convey semantic meaning (e.g. red text for critical CPU usage). The priority model ensures
   semantic colors override configured colors. This is analogous to `icon_color` in `ViewData`.

3. **Should the `opacity` property in `.widget-info-text` CSS be removed?** — **No.** The `opacity: 0.8` in `.widget-info-text` remains as the default for
   unconfigured widgets. However, when a user configures `info_text_color` (or `main_text_color`), the dynamically generated CSS rule (GTK `CssProvider`) and
   the web inline style both include `opacity: 1;` to neutralise the CSS class `opacity`. This prevents the compounding effect where `rgba()` alpha and CSS
   `opacity` would multiply (e.g. `alpha=0.5` + `opacity=0.8` → effective `0.4` instead of the intended `0.5`). The user expects to see exactly the color they
   configured, not a dimmed version.

```css
/* Dynamic CssProvider / inline style when info_text_color is set */
.text-color-ff6600ff {
    color: rgba(255, 102, 0, 1.0);
    opacity: 1; /* Neutralises .widget-info-text { opacity: 0.8; } */
}
```

This applies to both `main_text_color` and `info_text_color` for consistency, even though `.widget-main-text` does not set `opacity`.

4. **Should atomic widgets support text color configuration?** — **Yes.** `AtomicWidgetConfig` should gain `WidgetTextColors` if it doesn't already have it. The
   atomic rendering pipeline (`render_atomic_graphic_default()`) must be extended to accept and use text colors. The `atomic_widget_impl!` macro must pass text
   colors through.

5. **Should `draw_label_text()` be extended or should a new function be created?** — **Extend.** Adding an `Option<Color>` parameter with `None` default
   preserves existing behaviour. Creating a separate function would lead to code duplication. This follows the minimal-change principle.

---

## 13. References

- `concepts/planned/WIDGET_ICON_COLOR.md` — Predecessor concept for `icon_color` configuration
- `docs/ICON_RENDERING.md` — Icon rendering architecture, `ViewData`, `AtomicGraphicData`, semantic coloring
- `plugin-api/src/widget/icon.rs` — `Color` struct, `WidgetIconRendering` trait, `css_class()` method
- `plugin-api/src/widget/icons.rs` — `WidgetIcon` struct, `deserialize_hex_color`, `serialize_hex_color`
- `plugin-api/src/widget/view_data.rs` — `ViewData` struct
- `plugin-api/src/atomic/graphic_data.rs` — `AtomicGraphicData` struct
- `plugin-api/src/atomic/graphic.rs` — Centralised atomic rendering pipeline (`render_atomic_graphic_default`)
- `plugin-api/src/atomic/macro.rs` — `atomic_widget_impl!` macro
- `plugin-api/src/atomic/build.rs` — `build_atomic_widget()`, `update_labels()`
- `plugin-api/src/nerd_font.rs` — `apply_icon_color()` for GTK `Image`
- `plugins/render-utils/src/drawing.rs` — `draw_label_text()`, `draw_text_centered()`
- `plugins/sysinfo/src/config.rs` — `SysinfoMultiWidgetConfig`, `PercentageWidgetConfig`
- `plugins/sysinfo/src/multi_widget.rs` — `SysinfoMultiWidget`, `render_view()`, `update_icon_display()`
- `plugins/sysinfo/src/graphic.rs` — `render_view_data_to_graphic()`
- `plugins/sysinfo/src/html.rs` — `render_view_data_to_html()`
- `plugins/sysinfo/src/atomic.rs` — `SysinfoAtomicWidget`, `SysinfoAtomicView::render()`
- `plugins/weather/src/atomic.rs` — `apply_atomic_icon_color()` pattern reference
- `resources/style.css` — `.widget-main-text`, `.widget-info-text` CSS rules
- `AGENTS.md` — Project conventions (one struct per file, trait implementations, documentation standards)
