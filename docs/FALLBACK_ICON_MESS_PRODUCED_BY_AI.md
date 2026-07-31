# Fallback Icon Inventory

Comprehensive list of all fallback icons used across the codebase, grouped by widget and mechanism.

## 1. Atomic Widget Macro — `default_icon` parameter

Passed to `atomic_widget_impl!` and used by `build_atomic_widget` for the GTK label, and by `render_atomic_graphic_default` when no custom icon is available.

| File                            | Line | Icon              | Codepoint  | Widget                |
|---------------------------------|------|-------------------|------------|-----------------------|
| `plugins/mpris/src/atomic.rs`   | 198  | `nf-fa-music`     | `\u{f001}` | MPRIS atomic widget   |
| `plugins/audio/src/atomic.rs`   | 162  | `nf-fa-volume_up` | `\u{f028}` | Audio atomic widget   |
| `plugins/weather/src/atomic.rs` | 460  | `nf-fa-folder`    | `\u{f07b}` | Weather atomic widget |

## 2. `resolve_icon_codepoint().unwrap_or()` — icon name resolution fallback

When `resolve_icon_codepoint` can't find the Nerd Font name in the lookup table, these chars are used as fallback.

| File                             | Line     | Icon              | Codepoint  |
|----------------------------------|----------|-------------------|------------|
| `plugins/mpris/src/atomic.rs`    | 103, 125 | `nf-fa-music`     | `\u{f001}` |
| `plugins/audio/src/atomic.rs`    | 82, 104  | `nf-fa-volume_up` | `\u{f028}` |
| `plugins/weather/src/atomic.rs`  | 135, 169 | `nf-fa-folder`    | `\u{f07b}` |
| `plugins/weather/src/graphic.rs` | 54       | `nf-fa-folder`    | `\u{f07b}` |

## 3. `AtomicGraphicData::error()` — error/loading state fallback

Returned when no status is available yet (loading) or when status indicates an error.

| File                            | Line | Icon              | Codepoint  | Text             |
|---------------------------------|------|-------------------|------------|------------------|
| `plugins/mpris/src/atomic.rs`   | 121  | `nf-fa-music`     | `\u{f001}` | "Loading..."     |
| `plugins/audio/src/atomic.rs`   | 100  | `nf-fa-volume_up` | `\u{f028}` | "Loading..."     |
| `plugins/weather/src/atomic.rs` | 154  | `nf-fa-folder`    | `\u{f07b}` | "Loading..."     |
| `plugins/weather/src/atomic.rs` | 159  | `nf-fa-folder`    | `\u{f07b}` | "Stale: {error}" |
| `plugins/weather/src/atomic.rs` | 164  | `nf-fa-folder`    | `\u{f07b}` | error message    |

## 4. `render_atomic_graphic_default` — GraphicOnly fallback

| File                               | Line    | Behaviour                                                                                                                                                                                    |
|------------------------------------|---------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `plugin-api/src/atomic/graphic.rs` | 116-117 | When `render_graphic` returns `false` in `GraphicOnly` mode, falls back to drawing `icon_char` (which comes from `render_atomic_graphic_data`, ultimately the `default_icon` from the macro) |

## 5. MPRIS Atomic Widget — `render_atomic_view` no-player fallback

| File                          | Line | Icon          | Text        |
|-------------------------------|------|---------------|-------------|
| `plugins/mpris/src/atomic.rs` | 142  | `nf-fa-music` | "No Player" |

## 6. MPRIS Non-Atomic Widget (`MprisWidget`) — GTK fallbacks

| File                           | Line | Icon                       | Context                                   |
|--------------------------------|------|----------------------------|-------------------------------------------|
| `plugins/mpris/src/graphic.rs` | 23   | `\u{f001}` (nf-fa-music)   | Status is `None` (loading)                |
| `plugins/mpris/src/graphic.rs` | 24   | `\u{f001}` (nf-fa-music)   | `!status.has_player`                      |
| `plugins/mpris/src/widget.rs`  | 141  | `audio-x-generic-symbolic` | `art_url` is a non-file URL               |
| `plugins/mpris/src/widget.rs`  | 144  | `audio-x-generic-symbolic` | `art_url` is `None`                       |
| `plugins/mpris/src/widget.rs`  | 170  | `nf-fa-music`              | No player active (compact mode)           |
| `plugins/mpris/src/widget.rs`  | 181  | `audio-x-generic-symbolic` | No player active (album art mode)         |
| `plugins/mpris/src/widget.rs`  | 451  | `audio-x-generic-symbolic` | Initial album art placeholder (wide mode) |
| `plugins/mpris/src/widget.rs`  | 426  | "No Player" (label)        | Wide mode label fallback                  |
| `plugins/mpris/src/widget.rs`  | 466  | "No Player" (label)        | Wide mode initial label                   |

## 7. MPRIS Atomic Graphic Renderer — album art fallback chain

`plugins/mpris/src/atomic_graphic.rs` — returns `false` (triggering `fallback_to_icon` in `render_atomic_graphic_default`, which draws `\u{f001}`) in these
cases:

| Line  | Condition                                    |
|-------|----------------------------------------------|
| 10-11 | No status available                          |
| 14-15 | No player active                             |
| 18-19 | No metadata                                  |
| 22-23 | No `art_url` in metadata                     |
| 33-34 | `draw_image_cover` failed to load image file |

## 8. Wallpaper Widget — fallback icon

| File                               | Line | Icon                                        | Context                   |
|------------------------------------|------|---------------------------------------------|---------------------------|
| `plugins/wallpaper/src/config.rs`  | 13   | `nf-md-wallpaper` (`DEFAULT_FALLBACK_ICON`) | Default constant          |
| `plugins/wallpaper/src/preview.rs` | 23   | `set_fallback_icon()`                       | Preview path is empty     |
| `plugins/wallpaper/src/preview.rs` | 31   | `set_fallback_icon()`                       | Preview file not found    |
| `plugins/wallpaper/src/preview.rs` | 49   | `set_fallback_icon()`                       | Preview image load failed |

## 9. Workspace Switcher — default icon

| File                                       | Line   | Icon            | Context                                       |
|--------------------------------------------|--------|-----------------|-----------------------------------------------|
| `plugins/workspace-switcher/src/config.rs` | 44, 66 | `nf-md-monitor` | Default icon for workspaces not in `icon_map` |

## 10. Network Widget — default icons

| File                            | Line | Constant                         | Icon                      |
|---------------------------------|------|----------------------------------|---------------------------|
| `plugins/network/src/config.rs` | 12   | `DEFAULT_ICON_WIFI_STRENGTH_4`   | `nf-md-wifi_strength_4`   |
| `plugins/network/src/config.rs` | 14   | `DEFAULT_ICON_WIFI_STRENGTH_3`   | `nf-md-wifi_strength_3`   |
| `plugins/network/src/config.rs` | 16   | `DEFAULT_ICON_WIFI_STRENGTH_2`   | `nf-md-wifi_strength_2`   |
| `plugins/network/src/config.rs` | 18   | `DEFAULT_ICON_WIFI_STRENGTH_1`   | `nf-md-wifi_strength_1`   |
| `plugins/network/src/config.rs` | 20   | `DEFAULT_ICON_WIFI_STRENGTH_OFF` | `nf-md-wifi_strength_off` |
| `plugins/network/src/config.rs` | 22   | `DEFAULT_ICON_ETHERNET_ON`       | `nf-md-network_outline`   |
| `plugins/network/src/config.rs` | 24   | `DEFAULT_ICON_ETHERNET_OFF`      | `nf-md-network_off`       |
| `plugins/network/src/config.rs` | 26   | `DEFAULT_ICON_VPN_ON`            | `nf-md-shield_key`        |
| `plugins/network/src/config.rs` | 28   | `DEFAULT_ICON_VPN_OFF`           | `nf-md-shield_off`        |
| `plugins/network/src/config.rs` | 30   | `DEFAULT_ICON_AIRPLANE_ON`       | `nf-md-airplane`          |
| `plugins/network/src/config.rs` | 32   | `DEFAULT_ICON_AIRPLANE_OFF`      | `nf-md-airplane_off`      |
| `plugins/network/src/config.rs` | 34   | `DEFAULT_ICON_THROUGHPUT`        | `nf-md-swap_vertical`     |
| `plugins/network/src/config.rs` | 36   | `DEFAULT_ICON_WIFI_SCAN`         | `nf-md-wifi_strength_4`   |
| `plugins/network/src/config.rs` | 38   | `DEFAULT_ICON_QR_CODE`           | `nf-md-qrcode`            |
| `plugins/network/src/widget.rs` | 410  | "Loading..." (label)             | Initial label             |

## 11. App Launcher — icon resolution fallback

| File                                  | Line | Behaviour                                                                                                                                |
|---------------------------------------|------|------------------------------------------------------------------------------------------------------------------------------------------|
| `plugins/app-launcher/src/graphic.rs` | 104  | `resolve_desktop_icon_path` returns `None` when no icon found → `render_graphic` returns `false` → fallback to `default_icon` from macro |

## Inconsistencies

The MPRIS widget alone has **5 different fallback paths**:

1. `\u{f001}` (nf-fa-music) — atomic widget macro default + `resolve_icon_codepoint` fallback + `AtomicGraphicData::error`
2. `"nf-fa-music"` — `render_atomic_view` no-player view
3. `"audio-x-generic-symbolic"` — non-atomic GTK widget album art fallback (4 locations in `widget.rs`)
4. `\u{f001}` — non-atomic `GraphicRenderer` loading/no-player state in `graphic.rs`
5. `\u{f04b}` / `\u{f04c}` / `\u{f04d}` — playback status icons (play/pause/stop)

The atomic and non-atomic MPRIS widgets use different fallback icon systems (Nerd Font codepoints vs GTK symbolic icon names), and the album art fallback uses a
GTK symbolic icon (`audio-x-generic-symbolic`) that is inconsistent with the Nerd Font icons used everywhere else.
