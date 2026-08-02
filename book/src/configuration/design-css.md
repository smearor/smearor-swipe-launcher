# Design and CSS

The launcher uses GTK-4 CSS for styling. The main stylesheet is at `resources/style.css` and is loaded via a `CssProvider`.

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

## CSS Classes

### Widget Classes

| Class           | Description                  |
|-----------------|------------------------------|
| `.menu-button`  | Standard menu button styling |
| `.close-button` | Close/back button styling    |
| `.glow-blue`    | Blue glow effect             |
| `.glow-green`   | Green glow effect            |

### Area Classes

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
