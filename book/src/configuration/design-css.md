# Design and CSS

The launcher uses GTK4 CSS for styling. CSS is applied in layers with increasing priority, so later layers override earlier ones.

## CSS Layers

| Layer        | Source                                           | Priority                              | Scope                                       |
|--------------|--------------------------------------------------|---------------------------------------|---------------------------------------------|
| Built-in     | `resources/style.css` (compiled in)              | `STYLE_PROVIDER_PRIORITY_APPLICATION` | All windows                                 |
| Global user  | `~/.config/smearor/style.css`                    | `STYLE_PROVIDER_PRIORITY_USER`        | All windows                                 |
| Per-instance | `{config_stem}.css` next to `{config_stem}.toml` | `STYLE_PROVIDER_PRIORITY_USER + 1`    | All windows (use `.instance-{id}` to scope) |

### Built-in CSS

The built-in stylesheet is compiled into the binary at build time. It provides the base theme, widget classes, and layout primitives. It cannot be overridden
directly — use the global user CSS layer instead.

### Global User CSS

If `~/.config/smearor/style.css` exists, it is loaded automatically at startup with `STYLE_PROVIDER_PRIORITY_USER`, overriding the built-in defaults. This is
the recommended way to customize the overall look and feel.

```css
/* ~/.config/smearor/style.css */
.menu-button {
    background-color: rgba(40, 40, 60, 0.9);
    border-radius: 8px;
}
```

### Per-Instance CSS

Each launcher instance can have its own CSS file. Given a config at `~/.config/smearor/launcher/my-launcher.toml`, the launcher looks for
`~/.config/smearor/launcher/my-launcher.css`. If found, it is loaded with `STYLE_PROVIDER_PRIORITY_USER + 1`, overriding both built-in and global user CSS.

> **Important**: GTK4 registers `CssProvider` instances at the `GdkDisplay` level, not per-window. Per-instance CSS affects **all** windows on the same display.
> To scope styles to a specific instance, prefix selectors with `.instance-{id}`:

```css
/* ~/.config/smearor/launcher/my-launcher.css */
.instance-my-launcher .menu-button {
    background-color: rgba(220, 0, 115, 0.2);
}
```

The `instance-{id}` CSS class is applied automatically to the instance's root window (see [Automatic CSS Classes](#automatic-css-classes) below).

### Hot-Reload

Both global and per-instance CSS files are watched for changes. When a file is modified, the old `CssProvider` is removed and a new one is loaded — all on the
GTK main thread. Changes appear within ~500ms (debounce delay).

- **Atomic saves** (temp file + rename) are handled correctly: the watcher detects the new file after the debounce interval.
- **Non-existent files**: If a CSS file does not exist at startup, the parent directory is watched. When the file is created, it is loaded automatically.
- **File deletion**: If a CSS file is deleted, its provider is removed from the display.

## Color Palette

The design uses a dark theme with vibrant accent colors:

| Color      | Hex       | Usage                 |
|------------|-----------|-----------------------|
| Background | `#1a1a2e` | Main background       |
| Surface    | `#16213e` | Elevated surfaces     |
| Primary    | `#dc0073` | Primary accent (pink) |
| Secondary  | `#00cc00` | Success / positive    |
| Tertiary   | `#ff6600` | Warning / attention   |
| Text       | `#e0e0e0` | Primary text          |
| Text muted | `#888888` | Secondary text        |

### Transparency

The launcher supports transparency via the layer-shell protocol. Background colors use alpha channels (e.g. `#1a1a2eff` for opaque, `#1a1a2e80` for 50%
transparent).

### Adaptive Colors

Widgets can read the system accent color via the [personalization service](../services/personalization.md) and adapt their icon colors accordingly.

### Theme Color Variables

The [theme service](../services/theme.md) injects CSS custom properties for the currently applied theme. These are available in all CSS layers:

| Variable          | Description             |
|-------------------|-------------------------|
| `--theme-color-1` | Primary accent color    |
| `--theme-color-2` | Secondary accent color  |
| `--theme-color-3` | Tertiary accent color   |
| `--theme-color-4` | Quaternary accent color |
| `--theme-color-5` | Quinary accent color    |

Each theme defines 5 colors per mode (dark/light) in `themes.toml`. See [theme service](../services/theme.md) for details.

## Automatic CSS Classes

The launcher applies CSS classes automatically based on context. These classes are always present and can be used in any CSS layer.

### Instance Classes

| Class            | Description                                                                                                                                     |
|------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| `.instance-{id}` | Applied to the instance's root window. The `{id}` is derived from the config filename stem (e.g. `my-launcher.toml` → `.instance-my-launcher`). |

### Area Classes

| Class        | Description                                                                                                                      |
|--------------|----------------------------------------------------------------------------------------------------------------------------------|
| `.area-{id}` | Applied to each area's root container. The `{id}` is the area name from the config (e.g. `[scroll_band]` → `.area-scroll_band`). |

### Widget Classes

| Class           | Description                                                                                                                                                 |
|-----------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `.widget-{id}`  | Applied to each widget's root element. The `{id}` is the plugin instance ID from the config (e.g. `{ id = "clock_widget", ... }` → `.widget-clock_widget`). |
| `.menu-button`  | Standard menu button styling                                                                                                                                |
| `.close-button` | Close/back button styling                                                                                                                                   |
| `.glow-blue`    | Blue glow effect                                                                                                                                            |
| `.glow-green`   | Green glow effect                                                                                                                                           |

## User-Configurable CSS Classes

### Area CSS Classes

Areas can apply custom CSS classes for background styling:

```toml
[games_area]
css_classes = ["games-area-bg"]
```

```css
.games-area-bg {
    background-color: rgba(26, 26, 46, 0.95);
    border-radius: 12px;
}
```

### Widget CSS Classes

Widgets can apply custom CSS classes via `WidgetLayout`:

```toml
[[scroll_band.plugins]]
id = "my_widget"
path = "target/release/libsmearor_my_widget.so"
css_classes = ["my-widget-highlight"]
```

```css
.my-widget-highlight {
    border: 2px solid #dc0073;
}
```

## Icon Colors

Icons can be colored per-widget via configuration:

```toml
icon_color = "#dc0073ff"
main_text_color = "#ff6600ff"
info_text_color = "#00cc00ff"
```

## Nerd Fonts

The launcher bundles Nerd Font symbols for icon rendering:

- `resources/JetBrainsMonoNLNerdFont/` — Full JetBrains Mono Nerd Font
- `resources/NerdFontsSymbolsOnly/` — Symbol-only font for smaller footprint

Icons are referenced by their Nerd Font name (e.g. `nf-md-volume_high`, `nf-fa-gamepad`).

## Per-Widget Icon Colors

Some widgets support `icon_color` to override the default icon color. This is applied via the `apply_icon_color` helper from `plugin-api::nerd_font`.

See [Icon Rendering](../features/icon-rendering.md) for the full icon system documentation.
