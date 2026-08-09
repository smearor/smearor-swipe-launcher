# Concept: Global GTK Widget Scaling Factor

This document describes the concept for a **global scaling factor** that uniformly scales all GTK widget dimensions in the Smearor Swipe Launcher, including
icon sizes, label heights, widget widths/heights, spacing, and CSS font sizes.

---

## 1. Motivation

The launcher's GTK widgets follow the **Unified 4-Line Layout** with fixed pixel heights:

| Line | Height      | Content            |
|------|-------------|--------------------|
| 0    | `icon_size` | Icon               |
| 1    | 20px        | `widget-main-text` |
| 2    | 16px        | `widget-info-text` |
| 3    | 16px        | spacer/bar         |

These dimensions are hardcoded as `i32` constants in `plugin-api/src/widget/` or set directly in builder functions. When users want larger or smaller widgets
(e.g. for high-DPI displays, different screen distances, or accessibility), they must manually adjust `icon_size`, `width`, and `height` per plugin instance in
`config.toml` — and even then, the fixed label heights (20px, 16px) and CSS font sizes cannot be adjusted at all.

A **global scaling factor** solves this by applying a single multiplier to all pixel-based dimensions and font sizes, configurable per launcher instance.
Individual widgets can **override** the global factor with their own `scale` value, allowing mixed scaling in the same layout.

---

## 2. Approach: Config-Injection (Option A)

The scaling factor is injected into each plugin's config JSON by the launcher core, following the same pattern already used for `wrapper.rotation` injection in
`SwipeLauncherConfig::plugin_config()`.

### Why not CSS-only scaling?

GTK4's `transform: scale()` via CSS does not adjust `width_request`, `height_request`, `exclusive_zone`, or touch hit-boxes. This leads to broken layouts where
the visual size changes but the allocated space does not.

### Why not GDK_SCALE / GDK_DPI_SCALE?

These are environment variables that affect the entire process globally. They cannot be configured per launcher instance and are not tunable from `config.toml`.

---

## 3. Configuration

### 3.1 Launcher Config (`config.toml`)

A new `scale` field is added to the `[launcher]` section:

```toml
[launcher]
scale = 1.5  # Global scaling factor (default: 1.0)
```

- Type: `f32`
- Default: `1.0` (no scaling)
- Range: `0.5` – `3.0` (values outside this range are clamped; NaN/infinity → `1.0`)
- Applies to: all GTK widget instances in this launcher instance

### 3.2 SwipeLauncherSettings

```rust
/// Minimum and maximum allowed scale values.
const SCALE_MIN: f32 = 0.5;
const SCALE_MAX: f32 = 3.0;

/// Clamps the scale to the valid range and guards against NaN/infinity.
///
/// Returns 1.0 for NaN, infinity, or values outside [SCALE_MIN, SCALE_MAX].
fn sanitize_scale(scale: f32) -> f32 {
    if scale.is_nan() || scale.is_infinite() {
        return 1.0;
    }
    scale.clamp(SCALE_MIN, SCALE_MAX)
}

/// Global scaling factor for GTK widget dimensions.
///
/// Multiplies all pixel-based widget dimensions (width, height, icon_size,
/// label heights, spacing) and CSS font sizes. Default is 1.0 (no scaling).
/// Values are clamped to [0.5, 3.0] on deserialization.
#[serde(default = "default_scale", deserialize_with = "deserialize_sanitized_scale")]
pub scale: f32,
```

The custom deserializer clamps the value during deserialization, so invalid values from `config.toml` never reach the widget layer:

```rust
fn deserialize_sanitized_scale<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = f32::deserialize(deserializer)?;
    Ok(sanitize_scale(raw))
}
```

---

## 4. Propagation

### 4.1 Injection in `plugin_config()`

In `smearor-swipe-launcher/src/config/launcher.rs`, the `plugin_config()` method already injects `wrapper.rotation` into each plugin's config JSON. The same
mechanism injects `scale`:

```rust
pub fn plugin_config(&self, id: &str) -> PluginConfig {
    let mut config = self.get_plugin_config(id).cloned().unwrap_or_else(|| {
        trace!("No config found for plugin {id}, using empty config");
        json!({})
    });

    // Existing: rotation injection
    let launcher_rotation = self.launcher.rotation.rotation().to_degrees();
    if let Some(wrapper) = config.get_mut("wrapper").and_then(|w| w.as_object_mut()) {
        let follows_rotation = wrapper.get("follows_rotation").and_then(|v| v.as_bool()).unwrap_or(false);
        let has_explicit_rotation = wrapper.get("rotation").is_some();
        if follows_rotation && !has_explicit_rotation && launcher_rotation != 0.0 {
            wrapper.insert("rotation".to_string(), json!(launcher_rotation));
        }
    }

    // NEW: scale injection — only inject if the plugin hasn't set its own scale
    let scale = self.launcher.scale;
    if scale != 1.0 && config.get("scale").is_none() {
        config["scale"] = json!(scale);
    }

    PluginConfig { config }
}
```

### 4.2 Widget Config Deserialization

Each widget crate's `Config` struct already flattens `WidgetDimensions` via `#[serde(flatten)]`. The per-widget `scale` field
(see [5.2](#52-plugin-apisrcwidgetdimensionsrs)) lives on `WidgetDimensions`, so it is automatically deserialized from the plugin config JSON — no new field
needed in any widget config struct.

The widget resolves the effective scale by preferring the per-widget value over the injected global value, then sanitizing:

```rust
let raw_scale = config.dimensions.scale.unwrap_or(global_scale);
let scale = sanitize_scale(raw_scale);
```

Where `global_scale` is the value injected by the launcher into the plugin config JSON (default `1.0`). The widget passes `scale` to all builder functions and
dimension calculations.

---

## 5. Affected Components

### 5.1 `plugin-api/src/widget/icons.rs`

`WidgetIcon` gets a scaled accessor:

```rust
impl WidgetIcon {
    /// Returns the icon size, scaled by the given factor.
    pub fn icon_size_scaled(&self, scale: f32) -> i32 {
        ((self.icon_size as f32) * scale).round() as i32
    }
}
```

### 5.2 `plugin-api/src/widget/dimensions.rs`

`WidgetDimensions` gets a new `scale: Option<f32>` field for per-widget override, plus scaled accessors:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, TypedBuilder)]
#[serde(default)]
pub struct WidgetDimensions {
    #[builder(default, setter(into))]
    pub width: Option<i32>,

    #[builder(default, setter(into))]
    pub height: Option<i32>,

    #[builder(default, setter(into))]
    pub max_width: Option<i32>,

    /// Per-widget scaling factor that overrides the global `[launcher]` scale.
    ///
    /// When `None`, the widget uses the global scale injected by the launcher.
    /// When `Some(value)`, this value replaces the global scale for this widget
    /// only — it is NOT multiplied on top of the global scale.
    ///
    /// Affects all pixel-based dimensions: width, height, max_width, icon_size,
    /// label heights (20px/16px), spacing, and CSS font sizes.
    ///
    /// Values are sanitized via `sanitize_scale()` (clamped to [0.5, 3.0],
    /// NaN/infinity → 1.0) before use. See Section 3.2.
    #[builder(default, setter(into))]
    #[serde(default)]
    pub scale: Option<f32>,
}
```

`WidgetDimensions` is already `#[serde(flatten)]`-ed into every widget config struct, so adding the `scale` field here requires no changes to individual widget
config structs. A dedicated `WidgetScale` struct for a single field would require adding `#[serde(flatten)]` in all 12+ widget crates — not worth it until more
scale-related fields are added (e.g. `font_scale`, `icon_scale`).

Scaled accessors:

```rust
impl WidgetDimensions {
    /// Returns the width, scaled by the given factor.
    pub fn width_scaled(&self, scale: f32) -> i32 {
        ((self.width.unwrap_or(DEFAULT_WIDGET_WIDTH) as f32) * scale).round() as i32
    }

    /// Returns the height, scaled by the given factor.
    pub fn height_scaled(&self, scale: f32) -> i32 {
        ((self.height.unwrap_or(DEFAULT_WIDGET_HEIGHT) as f32) * scale).round() as i32
    }

    /// Returns the max width, scaled by the given factor.
    pub fn max_width_scaled(&self, mode: WidgetMode, scale: f32) -> i32 {
        let default = match mode {
            WidgetMode::Wide => DEFAULT_WIDE_MODE_WIDGET_WIDTH,
            WidgetMode::Compact => DEFAULT_WIDGET_WIDTH,
        };
        ((self.max_width.unwrap_or(default) as f32) * scale).round() as i32
    }

    /// Returns the effective widget width: `min(width, max_width)`, both scaled.
    pub fn effective_width_scaled(&self, mode: WidgetMode, scale: f32) -> i32 {
        self.width_scaled(scale).min(self.max_width_scaled(mode, scale))
    }

    /// Builds a `Button` with scaled dimensions.
    pub fn build_button_scaled(&self, mode: WidgetMode, content: &impl IsA<Widget>, max_width_css_prefix: &str, scale: f32) -> Button {
        let builder = Button::builder()
            .css_classes(["scroll-item", "menu-button"])
            .width_request(self.effective_width_scaled(mode, scale))
            .height_request(self.height_scaled(scale))
            .child(content);

        if let Some(max_w) = self.max_width {
            let scaled_max_w = ((max_w as f32) * scale).round() as i32;
            let css_class = format!("{}{}", max_width_css_prefix, scaled_max_w);
            let builder = builder
                .hexpand(false)
                .halign(Align::Start)
                .css_classes(["scroll-item", "menu-button", css_class.as_str()]);
            let css = format!(".{}{} {{ max-width: {}px; }}", max_width_css_prefix, scaled_max_w, scaled_max_w);
            register_css_once(&css_class, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            builder.build()
        } else {
            builder.build()
        }
    }
}
```

### 5.3 `plugin-api/src/widget/layout.rs`

`WidgetLayout` gets a scaled accessor:

```rust
impl WidgetLayout {
    /// Returns the spacing, scaled by the given factor.
    pub fn spacing_scaled(&self, scale: f32) -> i32 {
        ((self.spacing.unwrap_or(DEFAULT_WIDGET_SPACING) as f32) * scale).round() as i32
    }
}
```

### 5.4 `plugin-api/src/widget/builders.rs`

All builder functions get a `scale: f32` parameter. The existing functions are kept as thin wrappers with `scale = 1.0` for backward compatibility:

```rust
/// Main text label height in pixels (unscaled).
const MAIN_LABEL_HEIGHT: f32 = 20.0;
/// Info text label height in pixels (unscaled).
const INFO_LABEL_HEIGHT: f32 = 16.0;

pub fn build_main_label(text: &str, text_color: Option<Color>, ellipsize: bool, max_width_chars: Option<i32>, scale: f32) -> Label {
    let mut builder = Label::builder().label(text).css_classes(["widget-main-text"]);
    if ellipsize {
        builder = builder.ellipsize(EllipsizeMode::End);
    }
    if let Some(chars) = max_width_chars {
        builder = builder.max_width_chars(chars);
    }
    let label = builder.build();
    label.set_height_request((MAIN_LABEL_HEIGHT * scale).round() as i32);
    apply_text_color(&label, text_color);
    label
}

pub fn build_info_label(text: &str, text_color: Option<Color>, ellipsize: bool, max_width_chars: Option<i32>, scale: f32) -> Label {
    let mut builder = Label::builder().label(text).css_classes(["widget-info-text"]);
    if ellipsize {
        builder = builder.ellipsize(EllipsizeMode::End);
    }
    if let Some(chars) = max_width_chars {
        builder = builder.max_width_chars(chars);
    }
    let label = builder.build();
    label.set_height_request((INFO_LABEL_HEIGHT * scale).round() as i32);
    apply_text_color(&label, text_color);
    label
}

pub fn build_spacer(height: i32, scale: f32) -> Label {
    let spacer = Label::new(Some(""));
    spacer.set_height_request(((height as f32) * scale).round() as i32);
    spacer
}

pub fn build_widget_icon(icon_size: i32, icon_color: Option<Color>, setup_fn: impl FnOnce(&Image), scale: f32) -> Image {
    let icon = Image::new();
    icon.set_pixel_size(((icon_size as f32) * scale).round() as i32);
    icon.add_css_class("nerd-icon");
    setup_fn(&icon);
    if let Some(color) = icon_color {
        apply_icon_color(&icon, color);
    }
    icon
}
```

### 5.5 CSS Font Scaling

CSS font sizes are defined in `resources/style.css` as fixed pixel values:

| CSS Class           | Font Size |
|---------------------|-----------|
| `.widget-main-text` | 14px      |
| `.widget-info-text` | 10px      |
| `.nerd-icon`        | 1.5em     |
| `.clock-time`       | 32px      |
| `.sysinfo-icon`     | 1.5em     |

CSS classes like `.widget-main-text` apply **display-wide** — a global `CssProvider` cannot target individual widgets. This creates a problem with per-widget
scale overrides: a widget with `scale = 2.0` would have doubled GTK dimensions but unscaled fonts, resulting in huge boxes with tiny text.

#### Solution: Two-Tier CSS Scaling

**CSS Provider Deduplication:**

All dynamic CSS registration goes through a single deduplication helper to avoid accumulating duplicate `CssProvider` instances on the display during widget
rebuilds:

```rust
use std::collections::HashSet;
use std::sync::OnceLock;

static REGISTERED_CSS: OnceLock<std::sync::Mutex<HashSet<String>>> = OnceLock::new();

/// Registers a CSS rule on the display exactly once per unique key.
///
/// Subsequent calls with the same key are no-ops. This prevents CssProvider
/// accumulation when widgets are rebuilt (layout changes, config reloads, etc.).
fn register_css_once(key: &str, css: &str, priority: u32) {
    let set = REGISTERED_CSS.get_or_init(|| std::sync::Mutex::new(HashSet::new()));
    let mut guard = set.lock().unwrap();
    if guard.insert(key.to_string()) {
        if let Some(display) = gdk::Display::default() {
            let provider = CssProvider::new();
            provider.load_from_string(css);
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                priority,
            );
        }
    }
}
```

This helper is used by both `build_button_scaled` (max-width CSS, priority `APPLICATION`) and `apply_widget_scaled_css` (per-widget font scaling, priority
`APPLICATION + 2`). The `key` is the unique CSS class name (e.g. `max-w-150`, `scale-200`), ensuring each rule is registered at most once per process lifetime.

**Tier 1 — Global scale (launcher-wide):**

The launcher core generates a CSS string with scaled font sizes and registers it once during initialization. This affects all widgets uniformly:

```rust
fn apply_global_scaled_css(scale: f32) {
    if scale == 1.0 {
        return;
    }
    let css = format!(
        ".widget-main-text {{ font-size: {}px; }}
         .widget-info-text {{ font-size: {}px; }}
         .nerd-icon {{ font-size: {}em; }}
         .clock-time {{ font-size: {}px; }}
         .sysinfo-icon {{ font-size: {}em; }}",
        (14.0 * scale).round(),
        (10.0 * scale).round(),
        (1.5 * scale as f64),
        (32.0 * scale).round(),
        (1.5 * scale as f64),
    );
    if let Some(display) = gdk::Display::default() {
        let provider = CssProvider::new();
        provider.load_from_string(&css);
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
    }
}
```

**Tier 2 — Per-widget override (scoped CSS class):**

When a widget has a per-widget `scale` that differs from the global scale, the widget generates a **scoped CSS rule** and adds a corresponding CSS class to its
root container. This overrides the global font sizes for just that widget's subtree:

```rust
/// Generates a scoped CSS class for per-widget font scaling and adds it to the widget's root container.
///
/// Returns the CSS class name (e.g. "scale-200") that was added to the container.
fn apply_widget_scaled_css(container: &impl IsA<Widget>, scale: f32) -> String {
    let class_name = format!("scale-{}", (scale * 100.0).round() as i32);
    let css = format!(
        ".{class_name} .widget-main-text {{ font-size: {}px; }}
         .{class_name} .widget-info-text {{ font-size: {}px; }}
         .{class_name} .nerd-icon {{ font-size: {}em; }}
         .{class_name} .clock-time {{ font-size: {}px; }}
         .{class_name} .sysinfo-icon {{ font-size: {}em; }}",
        (14.0 * scale).round(),
        (10.0 * scale).round(),
        (1.5 * scale as f64),
        (32.0 * scale).round(),
        (1.5 * scale as f64),
    );
    register_css_once(&class_name, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 2);
    container.add_css_class(&class_name);
    class_name
}
```

The widget calls this during build when its effective scale differs from the global scale:

```rust
let effective_scale = config.dimensions.scale.unwrap_or(global_scale);
if effective_scale != global_scale {
apply_widget_scaled_css( & button, effective_scale);
}
```

The scoped class (e.g. `.scale-200 .widget-main-text`) has higher CSS specificity than the global rule (`.widget-main-text`). Additionally, the per-widget
provider is registered at priority `APPLICATION + 2`, which is higher than the global scale CSS (`APPLICATION + 1`). In GTK4, provider priority takes precedence
over CSS selector specificity, so the higher priority ensures per-widget overrides always win.

#### CSS Provider Lifecycle

| Provider                       | Priority          | Scope          | When                              | Deduplicated                                 |
|--------------------------------|-------------------|----------------|-----------------------------------|----------------------------------------------|
| Base `style.css`               | `APPLICATION`     | Display-wide   | Startup                           | Static file                                  |
| Global scale CSS               | `APPLICATION + 1` | Display-wide   | Startup (if `scale != 1.0`)       | Once (startup)                               |
| Max-width CSS (`build_button`) | `APPLICATION`     | Display-wide   | Widget build                      | `register_css_once` (per unique class)       |
| Per-widget scoped CSS          | `APPLICATION + 2` | Widget subtree | Widget build (if override active) | `register_css_once` (per unique scale class) |

All dynamic providers use `register_css_once` with a unique key (the CSS class name). Each rule is registered at most once per process lifetime, regardless of
how many times widgets are rebuilt. The scoped class on the container ensures only that widget's subtree is affected.

---

## 6. Widget Migration

Each widget crate must:

1. Resolve and sanitize the effective scale: `let scale = sanitize_scale(config.dimensions.scale.unwrap_or(global_scale));`
2. Pass `scale` to all builder function calls
3. Use scaled dimension accessors where applicable
4. If `effective_scale != global_scale`, call `apply_widget_scaled_css(&button, effective_scale)` to scope font sizes to this widget

### Affected Widget Crates

| Crate                        | Builder Calls | Notes                                                                                                         |
|------------------------------|---------------|---------------------------------------------------------------------------------------------------------------|
| `plugins/button`             | 4             | Uses `build_main_label`, `build_info_label`, `build_widget_icon`, `build_spacer`                              |
| `plugins/clock`              | 4             | Uses `build_spacer`, plus hardcoded `set_height_request(20)` and `set_height_request(16)` that must be scaled |
| `plugins/audio`              | 12            | Uses all builder functions, plus volume bar                                                                   |
| `plugins/mpris`              | 12            | Uses all builder functions, plus progress bar                                                                 |
| `plugins/network`            | 10            | Uses all builder functions                                                                                    |
| `plugins/power`              | 10            | Uses all builder functions, plus timeout bar                                                                  |
| `plugins/doa`                | 8             | Uses all builder functions                                                                                    |
| `plugins/wallpaper`          | 8             | Uses all builder functions                                                                                    |
| `plugins/workspace-switcher` | 8             | Uses all builder functions                                                                                    |
| `plugins/weather`            | 4             | Uses all builder functions                                                                                    |
| `plugins/app-launcher`       | 2             | Uses `build_content_box`                                                                                      |
| `plugins/voice_assistant`    | 2             | Uses `build_content_box`                                                                                      |
| `plugins/sysinfo`            | —             | Uses `sysinfo-icon` CSS class (covered by CSS scaling)                                                        |

### Clock Widget Special Case

The clock widget (`plugins/clock/src/widget.rs`) has hardcoded `set_height_request` calls that bypass the builder functions:

```rust
// Line 194: time_label uses icon_size (already scaled via WidgetIcon)
time_label.set_height_request(config.icon_config.icon_size());

// Line 203: date_label uses hardcoded 20
date_label.set_height_request(20);

// Line 212: weekday_label uses hardcoded 16
weekday_label.set_height_request(16);

// Line 217: spacer uses hardcoded 16
let spacer = build_spacer(16);
```

These must be changed to use `scale`:

```rust
time_label.set_height_request(config.icon_config.icon_size_scaled(scale));
date_label.set_height_request((20.0 * scale).round() as i32);
weekday_label.set_height_request((16.0 * scale).round() as i32);
let spacer = build_spacer(16, scale);
```

---

## 7. Implementation Phases

### Phase 1: plugin-api Scaled Accessors

**Scope:** Add `scale: Option<f32>` field to `WidgetDimensions`, add scaled methods to `WidgetIcon`, `WidgetDimensions`, `WidgetLayout`, add `register_css_once`
deduplication helper, and update builder function signatures.

**Files:**

- `plugin-api/src/widget/icons.rs` — add `icon_size_scaled(scale: f32) -> i32`
- `plugin-api/src/widget/dimensions.rs` — add `scale: Option<f32>` field, add `width_scaled`, `height_scaled`, `max_width_scaled`, `effective_width_scaled`,
  `build_button_scaled` (using `register_css_once`)
- `plugin-api/src/widget/layout.rs` — add `spacing_scaled(scale: f32) -> i32`
- `plugin-api/src/widget/builders.rs` (or a new `css.rs`) — add `register_css_once(key, css)` helper, add `scale: f32` parameter to `build_main_label`,
  `build_info_label`, `build_spacer`, `build_widget_icon`, `build_content_box`

**Exit Criteria:** All scaled methods compile. `WidgetDimensions.scale` deserializes correctly from TOML. `register_css_once` prevents duplicate `CssProvider`
registration. Existing unscaled methods remain as wrappers with `scale = 1.0`.

### Phase 2: Launcher Config & Injection

**Scope:** Add `scale` field to `SwipeLauncherSettings` and inject it into plugin configs.

**Files:**

- `smearor-swipe-launcher/src/config/launcher.rs` — add `scale: f32` to `SwipeLauncherSettings`, inject in `plugin_config()`
- `configs/launcher/config.toml` — add example `scale = 1.0` entry with documentation

**Exit Criteria:** `scale` value from `[launcher]` section appears in every plugin's config JSON when not 1.0.

### Phase 3: CSS Font Scaling

**Scope:** Implement two-tier CSS font scaling — global scale CSS at startup, plus a per-widget scoped CSS helper in `plugin-api`.

**Files:**

- `smearor-swipe-launcher/src/` (window initialization) — call `apply_global_scaled_css(scale)` after main CSS provider is loaded
- `plugin-api/src/widget/builders.rs` (or a new `css.rs`) — add `apply_widget_scaled_css(container, scale)` helper for per-widget overrides

**Exit Criteria:** With `scale = 1.5`, font sizes in `.widget-main-text`, `.widget-info-text`, `.nerd-icon`, `.clock-time`, and `.sysinfo-icon` are visually
larger. With a per-widget `scale = 2.0` overriding a global `1.0`, that widget's fonts are doubled while others remain unscaled.

### Phase 4: Widget Migration

**Scope:** Update each widget crate to resolve the effective scale (`config.dimensions.scale.unwrap_or(global_scale)`) and pass it to all builder calls.

**Order (by complexity):**

1. `plugins/button` (simplest, 4 calls)
2. `plugins/clock` (special case: hardcoded heights)
3. `plugins/weather`
4. `plugins/app-launcher`
5. `plugins/voice_assistant`
6. `plugins/doa`
7. `plugins/wallpaper`
8. `plugins/workspace-switcher`
9. `plugins/network`
10. `plugins/power`
11. `plugins/audio`
12. `plugins/mpris`

**Exit Criteria:** With `scale = 1.5`, all widgets render with proportionally larger dimensions and font sizes.

### Phase 5: Testing & Validation

- Verify `scale = 1.0` produces identical output to current behavior (no regression)
- Verify `scale = 0.0`, negative, NaN, and infinity values are clamped to valid range (no GTK panics)
- Verify `scale = 0.5` produces usable small widgets
- Verify `scale = 2.0` produces usable large widgets
- Verify per-widget `scale` override takes precedence over global `[launcher]` scale
- Verify per-widget `scale` override also scales CSS font sizes (scoped class applied, not just GTK dimensions)
- Verify `max_width` CSS scaling works correctly in Wide mode
- Verify `exclusive_zone` interaction (area dimensions may need manual adjustment)
- Verify atomic widgets (Stream Deck, Loupedeck) are unaffected (they use physical dimensions, not GTK pixel sizes)
- Verify no `CssProvider` accumulation: rebuild a widget multiple times and confirm `register_css_once` does not create duplicate providers

### Phase 6: Documentation

**Scope:** Document the `scale` configuration option by integrating it into existing documentation surfaces — no separate book page needed.

**Files:**

- `book/src/configuration/launcher-config.md` — add `scale` row to the `[launcher]` settings table; add a short "Widget Scaling" subsection below the table
  explaining the effect on the Unified 4-Line Layout, what is NOT scaled, and recommended value ranges
- `configs/launcher/config.toml` — add inline comment documenting `scale` with example values and caveats (area dimensions and `exclusive_zone` not auto-scaled)
- `docs/CONCEPT_SKILL.md` — update the Unified 4-Line Layout section to note that all pixel values are multiplied by the global `scale` factor when configured

**Content for `launcher-config.md` — table row:**

| `scale` | `f32` | `1.0` | Global widget scaling factor (multiplies all pixel dimensions and font sizes) |

**Content for `launcher-config.md` — subsection:**

```markdown
### Widget Scaling

The `scale` field in `[launcher]` applies a global multiplier to all GTK widget dimensions (width, height, icon size, label heights, spacing) and CSS font
sizes.

Recommended range: `0.5`–`3.0`. Common use cases:

| Scale  | Use Case                        |
|--------|---------------------------------|
| `1.0`  | Default (no scaling)            |
| `1.5`  | High-DPI displays (4K)          |
| `2.0`  | Accessibility / large widgets   |
| `0.75` | Compact layouts                 |

**Not scaled:** Area dimensions (`width`, `spacing` in `[area]` sections),
`exclusive_zone`, and atomic widgets (Stream Deck, Loupedeck). Adjust these manually if needed.

Per-widget `width`, `height`, and `icon_size` configs are multiplied by `scale`
on top of their configured values.

A per-widget `scale` in the plugin's own config overrides the global value.
```

**Exit Criteria:** `scale` is documented in the launcher-config book page, config file, and concept skill. A user can discover and understand the feature
without reading source code.

---

## 8. Backward Compatibility

- `scale` defaults to `1.0` — no behavior change unless explicitly configured
- Existing unscaled builder methods are kept as wrappers (`scale = 1.0`)
- Per-widget `width`, `height`, `icon_size` configs still work as before; the scale factor is an additional multiplier on top
- Per-widget `scale` override replaces (not multiplies) the global scale for that widget
- Atomic widgets are unaffected — they derive icon size from physical button dimensions, not from `WidgetIcon` or `WidgetDimensions`

---

## 9. Limitations

- **Area dimensions**: `AreaConfig` (`width`, `width_percent`, `min_width`, `max_width`, `spacing`) is not scaled by this factor. Users may need to manually
  adjust area dimensions to accommodate scaled widgets. A future extension could apply the scale factor to area dimensions as well.
- **`exclusive_zone`**: Not automatically scaled. Users must adjust this manually in `[launcher]`.
- **Bar widgets**: Volume bars, progress bars, and timeout bars use `min-height` in CSS (e.g. `min-height: 8px`). These would need additional CSS scaling rules
  or `set_height_request` adjustments.
- **Non-GTK rendering**: Atomic widgets (Stream Deck, Loupedeck, Respeaker) use `render_graphic()` with physical pixel dimensions. These are intentionally not
  affected by the GTK scale factor.

---

## 10. Config Example

```toml
[launcher]
scale = 1.5  # global scale applied to all widgets

[my_small_widget]
scale = 1.0  # this widget stays at default size despite global 1.5

[my_large_widget]
scale = 2.0  # this widget is even larger than the global 1.5
```

---

## 11. Estimated Effort

| Component                    | Lines Changed | Complexity |
|------------------------------|---------------|------------|
| `plugin-api` scaled methods  | ~90           | Low        |
| Launcher config & injection  | ~15           | Low        |
| CSS font scaling             | ~25           | Low        |
| Widget migration (12 crates) | ~120          | Medium     |
| Documentation (Phase 6)      | ~40           | Low        |
| **Total**                    | **~290**      | **Medium** |
