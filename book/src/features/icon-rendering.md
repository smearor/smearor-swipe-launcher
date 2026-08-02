# Icon Rendering

The launcher provides a flexible icon rendering system that supports Nerd Font icons, GTK icon theme icons, dynamic state-dependent icons, and view-dependent
icons.

## Configuration Fields

| Field        | Type             | Description                                              |
|--------------|------------------|----------------------------------------------------------|
| `icon`       | `Option<String>` | Icon name (Nerd Font or GTK icon theme)                  |
| `icon_size`  | `i32`            | Icon size in pixels                                      |
| `icon_only`  | `bool`           | Show only the icon, hide the text label                  |
| `icon_color` | `String`         | Hex color for the icon (e.g. `#dc0073ff`)                |
| `mode`       | `WidgetMode`     | Layout mode: `compact` (vertical) or `wide` (horizontal) |
| `max_width`  | `Option<i32>`    | Maximum widget width in pixels                           |
| `show_icon`  | `bool`           | Whether to show an icon at all                           |

## Icon Sources

1. **Nerd Font icons** — Referenced by name (e.g. `nf-md-volume_high`, `nf-fa-gamepad`)
2. **GTK icon theme** — Standard Freedesktop icon names
3. **Dynamic icons** — Resolved at runtime based on widget state or view

## Dynamic Icon Categories

### State-Dependent Icons

Icons that change based on the widget's current state. For example:

- **audio** — Volume icon changes with mute state and volume level
- **network** — WiFi signal strength icon changes with signal quality
- **mpris** — Play/pause icon changes with playback state

### View-Dependent Icons

Widgets that cycle through multiple views (via swipe up/down) can have different icons per view:

- **power** — Each power action (shutdown, reboot, suspend, etc.) has its own icon
- **network** — 7 views (WiFi, Ethernet, Throughput, Scan, VPN, Airplane, QR)
- **weather** — 15 views (Current, Forecast, Wind, UV, etc.)
- **sysinfo-multi** — 9 views (CPU, Memory, Disk, Network, etc.)

## Widget Icon Matrix

| Widget          | Static Icon | Dynamic Icon | View-Dependent | State-Dependent |
|-----------------|:-----------:|:------------:|:--------------:|:---------------:|
| app-launcher    |     ✅      |      —       |       —        |        —        |
| button          |     ✅      |      ✅      |       —        |       ✅        |
| audio           |      —      |      ✅      |       —        |       ✅        |
| mpris           |      —      |      ✅      |       —        |       ✅        |
| power           |      —      |      ✅      |       ✅       |        —        |
| network         |      —      |      ✅      |       ✅       |       ✅        |
| wallpaper       |      —      |      ✅      |       ✅       |       ✅        |
| weather         |      —      |      ✅      |       ✅       |       ✅        |
| sysinfo-multi   |      —      |      ✅      |       ✅       |       ✅        |
| clock           |      —      |      —       |       —        |        —        |
| voice_assistant |      —      |      ✅      |       —        |       ✅        |

## Text Colors

Widgets support configurable text colors:

```toml
main_text_color = "#ff6600ff"
info_text_color = "#00cc00ff"
```

Both accept hex color strings (e.g. `#ff6600`, `#f60`, `#ff660080`). If omitted, the default text color from the theme is used.
