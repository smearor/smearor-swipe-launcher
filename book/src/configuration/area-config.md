# Area Configuration

Areas define the layout regions of the launcher. Each area contains a set of plugins and has a type (fixed or scroll).

## Area Types

### Fixed Area

A static area with a fixed width:

```toml
[left_area]
area_type = "fixed"
width = 200
align = "right"
plugins = [
    { id = "clock_widget", path = "target/release/libsmearor_clock_widget.so" }
]
```

| Field           | Type               | Description                     |
|-----------------|--------------------|---------------------------------|
| `area_type`     | `String`           | Must be `"fixed"`               |
| `width`         | `Option<i32>`      | Fixed width in pixels           |
| `width_percent` | `Option<f32>`      | Width as percentage of total    |
| `align`         | `String`           | `"left"`, `"right"`, `"center"` |
| `plugins`       | `Vec<PluginEntry>` | Plugins in this area            |

### Scroll Area

A scrollable area with drag gesture support:

```toml
[scroll_band]
area_type = "scroll"
spacing = 0
plugins = [
    { id = "clock_widget", path = "target/release/libsmearor_clock_widget.so" },
    { id = "audio", path = "target/release/libsmearor_audio_widget.so", widget = "audio" }
]
```

| Field       | Type               | Description             |
|-------------|--------------------|-------------------------|
| `area_type` | `String`           | Must be `"scroll"`      |
| `spacing`   | `i32`              | Spacing between plugins |
| `hexpand`   | `bool`             | Expand horizontally     |
| `vexpand`   | `bool`             | Expand vertically       |
| `plugins`   | `Vec<PluginEntry>` | Plugins in this area    |

## Area Includes

Areas can include shared configuration from external files:

```toml
[games_area]
include = "../areas/scroll_menu.toml"
open_transition = "SlideUp"
css_classes = ["games-area-bg"]
plugins = [...]
```

The included file provides common properties; the area can override or add properties.

## Transient Areas

Transient areas auto-close when the user clicks outside or presses escape:

```toml
[popup_area]
area_type = "scroll"
transient = true
close_on_escape = true
open_transition = "Pop"
plugins = [...]
```

## Transition Animations

| Value        | Description       |
|--------------|-------------------|
| `None`       | No animation      |
| `Fade`       | Fade in/out       |
| `SlideLeft`  | Slide from left   |
| `SlideRight` | Slide from right  |
| `SlideUp`    | Slide from top    |
| `SlideDown`  | Slide from bottom |
| `Pop`        | Pop in/out        |
| `Scale`      | Scale in/out      |

## CSS Classes

Areas can apply CSS classes for styling:

```toml
[games_area]
css_classes = ["games-area-bg"]
```

See [Design and CSS](./design-css.md) for CSS styling details.
