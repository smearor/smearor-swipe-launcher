# `smearor-swipe-launcher`: Swipe-Driven Scrolling App Launcher

## Vision

A small ribbon at the bottom of the screen that you can swipe left and right with your fingers to navigate a

longer list of apps. Clicking on an app launches it. Supports rotation.

## Window

- Rust

- GTK-4 Window

    - Layer-Shell-Layer

    - No Decorations

    - Rotation also determines the position of the layer:

        - 0 Degrees: bottom

        - 90 Degrees: left

        - 180 Degrees: top

        - 270 Degrees: right

        - Child: RotationWidget

            - Child: SwipeWidget

                - Swipe Left (Touch or Mouse or SUPER+ARROW-KEY): Scroll left

                - Swipe Right (Touch or Mouse or SUPER+ARROW-KEY): Scroll right

                - Swipe Up (Touch or Mouse or SUPER+ARROW-KEY): Parent Menu

                - Swipe Down (Touch or Mouse or SUPER+ARROW-KEY): Scroll

                - Child: MenuEntryWidget (Persistent, left of scrollable)

                    - Child: MenuEntryWidget (Dynamic)

                        - Shows the desktop entry icon

                        - Shows the desktop entry name

                        - Shows the desktop entry description

                        - Click or Touch: Execute Primary Action

                            - Launch menu entry (if smearor-wrot: with --rotation parameter)

                        - Longpress or Longtouch: Execute Secondary Action

                - Child: MenuEntryWidget (Persistent, left of scrollable)

## MenuEntryWidget

In principle, the MenuEntryWidget renders a single GTK widget.

Challenge:

- Widget: Launch-Desktop-Entry

- Widget: Date / Time

- Widget: Notification

- Widget: MPRIS

These widgets should not be hard-coded, but loaded from a configuration file.

This is a fantastic and extremely well thought-out concept! A touch-optimized, rotatable launcher with smooth scrolling fills a real gap in the Wayland
ecosystem (especially for Linux tablets or convertibles).

Since you want to load the widgets completely dynamically via a configuration file, a simple match on predefined enums is not enough. You need a plugin or
component system where each type of widget encapsulates its own logic.

Here is the architectural plan and the concrete implementation of how to build this system in Rust and GTK 4.

## Flexible Layout System

The launcher now features a flexible layout system that allows dynamic area management through configuration and runtime API.

### Configuration Format

The layout is defined in `config.toml` with the following structure:

```toml
# Define the order of areas
areas = ["left_area", "scroll_band", "right_area"]

# Area configurations
[left_area]
area_type = "Fixed"
width = 200
transition = "Fade"
plugins = [{ id = "clock" }]

[scroll_band]
area_type = "Scroll"
transition = "SlideLeft"
plugins = [{ id = "app-launcher" }]

[right_area]
area_type = "Fixed"
width = 150
transition = "None"
plugins = [{ id = "battery" }]
```

### Area Types

- **Fixed**: Static area with fixed width
    - `width`: Fixed width in pixels
    - `width_percent`: Alternative width as percentage (optional)

- **Scroll**: Scrollable area with drag gesture
    - `hexpand`: Horizontal expansion (default: true)
    - `vexpand`: Vertical expansion (default: true)

### Transition Animations

Areas support smooth transition animations:

- `None` - No animation
- `Fade` - Fade in/out effect
- `SlideLeft` - Slide from left
- `SlideRight` - Slide from right
- `SlideUp` - Slide from top
- `SlideDown` - Slide from bottom
- `Pop` - Pop in/out effect
- `Scale` - Scale in/out effect

### Dynamic Area API

The `AreaManager` provides runtime area management:

```rust
// Add a static area
area_manager.add_area_from_config("my_area", area_config, & main_container) ?;

// Add a transient area (auto-closing)
area_manager.add_transient_area("popup_menu", area_config, & main_container) ?;

// Remove an area
area_manager.remove_area("my_area", & main_container) ?;

// Pop from area stack (for nested sub-menus)
area_manager.pop_area( & main_container) ?;

// Get area information
if let Some(area) = area_manager.get_area("my_area") {
println ! ("Area: {}", area.id);
}
```

### Transient Areas

Transient areas automatically close when:

- User clicks outside the area
- Escape key is pressed (if `close_on_escape` is enabled)

### Nested Sub-Menus

The `AreaStack` manages nested sub-menus:

- Push areas onto the stack when opening sub-menus
- Pop areas when closing sub-menus
- Supports unlimited nesting depth

### Plugin Integration

Plugins are automatically loaded into areas:

- Each area can contain multiple plugins
- Plugins are loaded from the configuration
- Plugin lifecycle is managed by the `PluginManager`


