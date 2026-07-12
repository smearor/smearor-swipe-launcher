# Button Widget Concept

## Overview

The Button widget provides a configurable button that can trigger dynamic messages via the plugin message system. It supports both single-click and long-press
actions, similar to the AppLauncher widget, but with configurable text and message payloads.

## Use Cases

- Opening sub-menus (e.g., "games" menu)
- Triggering custom actions
- Sending commands to other plugins
- Dynamic UI state changes

## Configuration

### Button Configuration Structure

```toml
[button_id]
text = "Games"
icon = "applications-games"
tooltip = "Open games menu"

# Display mode
icon_only = false

# Single-click action
click_topic = "area.open"
click_payload = { area_id = "games_area" }

# Long-press action
longpress_topic = "area.close"
longpress_payload = { area_id = "games_area" }

# Swipe-up action
swipe_up_topic = "area.next"
swipe_up_payload = { area_id = "games_area" }

# Swipe-down action
swipe_down_topic = "area.prev"
swipe_down_payload = { area_id = "games_area" }

# Keyboard shortcut
shortcut = "Ctrl+G"

# Button state
enabled = true
active = false

# Animation
press_animation = "scale"

# Styling
css_classes = ["menu-button", "primary"]
```

### Configuration Parameters

- **text** (required): Button label text (hidden if icon_only is true)
- **icon** (optional): Icon name from icon theme
- **tooltip** (optional): Tooltip text on hover
- **icon_only** (optional, default: false): Show only icon without text
- **click_topic** (optional): Message topic for single-click
- **click_payload** (optional): Message payload for single-click (JSON/TOML)
- **click_instance** (optional): Target instance for single-click message
- **longpress_topic** (optional): Message topic for long-press
- **longpress_payload** (optional): Message payload for long-press (JSON/TOML)
- **longpress_instance** (optional): Target instance for long-press message
- **swipe_up_topic** (optional): Message topic for swipe-up gesture
- **swipe_up_payload** (optional): Message payload for swipe-up gesture (JSON/TOML)
- **swipe_up_instance** (optional): Target instance for swipe-up message
- **swipe_down_topic** (optional): Message topic for swipe-down gesture
- **swipe_down_payload** (optional): Message payload for swipe-down gesture (JSON/TOML)
- **swipe_down_instance** (optional): Target instance for swipe-down message
- **shortcut** (optional): Keyboard shortcut (e.g., "Ctrl+G", "Alt+F1")
- **enabled** (optional, default: true): Whether the button is interactive
- **active** (optional, default: false): Whether the button is in active state
- **press_animation** (optional): Animation type on button press (scale, fade, ripple)
- **css_classes** (optional): Additional CSS classes for styling

## Message System

### Message Format

The button sends messages through the plugin message system with the following structure:

```rust
pub struct ButtonMessage {
    pub topic: String,
    pub payload: serde_json::Value,
}
```

### Example Messages

**Single-click to open submenu:**

```json
{
  "topic": "area.open",
  "payload": {
    "area_id": "games_area"
  }
}
```

**Long-press to close submenu:**

```json
{
  "topic": "area.close",
  "payload": {
    "area_id": "games_area"
  }
}
```

**Add area dynamically:**

```json
{
  "topic": "area.add",
  "payload": {
    "area_id": "new_area",
    "area_config": {
      "area_type": "Scroll",
      "transition": "Fade",
      "plugins": [
        {
          "id": "some_plugin"
        }
      ]
    }
  }
}
```

**Remove area dynamically:**

```json
{
  "topic": "area.remove",
  "payload": {
    "area_id": "new_area"
  }
}
```

## Implementation

### Widget Structure

```rust
pub struct ButtonWidget {
    button: gtk4::Button,
    config: ButtonConfig,
    message_sender: tokio::sync::mpsc::Sender<FfiEnvelope>,
    shortcut_controller: Option<gtk4::EventControllerKey>,
}

pub struct ButtonConfig {
    pub text: String,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub icon_only: bool,
    pub click_topic: Option<String>,
    pub click_payload: Option<serde_json::Value>,
    pub longpress_topic: Option<String>,
    pub longpress_payload: Option<serde_json::Value>,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub active: bool,
    pub press_animation: Option<String>,
    pub css_classes: Vec<String>,
}
```

### Event Handlers

**Single-click handler:**

```rust
button.connect_clicked(move | _ | {
if ! config.enabled {
return;
}
if let Some(topic) = & config.click_topic {
let payload = config.click_payload.clone().unwrap_or(json ! ({}));
send_message(topic, payload);
}
});
```

**Long-press handler:**

```rust
let gesture = gtk4::GestureLongPress::new();
gesture.connect_pressed(move | _, _, _ | {
if ! config.enabled {
return;
}
if let Some(topic) = & config.longpress_topic {
let payload = config.longpress_payload.clone().unwrap_or(json ! ({}));
send_message(topic, payload);
}
});
button.add_controller(gesture);
```

**Keyboard shortcut handler:**

```rust
if let Some(shortcut) = & config.shortcut {
let key_controller = gtk4::EventControllerKey::new();
let shortcut_clone = shortcut.clone();
let topic_clone = config.click_topic.clone();
let payload_clone = config.click_payload.clone();

key_controller.connect_key_pressed( move | _, keyval, _, state | {
if parse_shortcut( & shortcut_clone, keyval, state) {
if let Some(topic) = & topic_clone {
let payload = payload_clone.clone().unwrap_or(json ! ({}));
send_message(topic, payload);
}
return gtk4::glib::Propagation::Stop;
}
gtk4::glib::Propagation::Proceed
});

button.add_controller(key_controller);
}
```

### Button State Management

**Enable/disable button:**

```rust
pub fn set_enabled(&self, enabled: bool) {
    self.button.set_sensitive(enabled);
    self.config.enabled = enabled;
}

pub fn is_enabled(&self) -> bool {
    self.config.enabled
}
```

**Active state management:**

```rust
pub fn set_active(&self, active: bool) {
    self.config.active = active;
    if active {
        self.button.add_css_class("active");
    } else {
        self.button.remove_css_class("active");
    }
}

pub fn is_active(&self) -> bool {
    self.config.active
}
```

### Icon-Only Mode

**Icon-only rendering:**

```rust
fn build_button_content(&self) {
    if self.config.icon_only {
        if let Some(icon_name) = &self.config.icon {
            let icon = gtk4::Image::from_icon_name(icon_name);
            icon.set_pixel_size(24);
            self.button.set_child(Some(&icon));
        }
    } else {
        let box_widget = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        if let Some(icon_name) = &self.config.icon {
            let icon = gtk4::Image::from_icon_name(icon_name);
            icon.set_pixel_size(24);
            box_widget.append(&icon);
        }

        let label = gtk4::Label::new(Some(&self.config.text));
        box_widget.append(&label);

        self.button.set_child(Some(&box_widget));
    }
}
```

### Press Animation

**Animation on button press:**

```rust
button.connect_pressed(move | _ | {
if let Some(animation) = & config.press_animation {
match animation.as_str() {
"scale" => {
button.set_scale(0.95, 0.95);
}
"fade" => {
button.set_opacity(0.7);
}
"ripple" => {
// Implement ripple effect
}
_ => {}
}
}
});

button.connect_released(move | _ | {
if let Some(animation) = & config.press_animation {
match animation.as_str() {
"scale" => {
button.set_scale(1.0, 1.0);
}
"fade" => {
button.set_opacity(1.0);
}
_ => {}
}
}
});
```

## Integration with AreaManager

The button widget integrates with the AreaManager to handle area-related messages:

### Area Open Message

When a button sends an `area.open` message:

```rust
if topic == "area.open" {
let area_id = payload["area_id"].as_str().unwrap();
let area_config = config.get_area_config(area_id).unwrap();
area_manager.add_transient_area(area_id, area_config, & main_container) ?;
}
```

### Area Close Message

When a button sends an `area.close` message:

```rust
if topic == "area.close" {
let area_id = payload["area_id"].as_str().unwrap();
area_manager.remove_area(area_id, & main_container) ?;
}
```

### Area Add Message

When a button sends an `area.add` message:

```rust
if topic == "area.add" {
let area_id = payload["area_id"].as_str().unwrap();
let area_config: AreaConfig = serde_json::from_value(payload["area_config"].clone()).unwrap();
area_manager.add_area_from_config(area_id, area_config, & main_container)?;
}
```

### Area Remove Message

When a button sends an `area.remove` message:

```rust
if topic == "area.remove" {
let area_id = payload["area_id"].as_str().unwrap();
area_manager.remove_area(area_id, & main_container) ?;
}
```

## Example Configuration

### Main Layout with Games Button

```toml
areas = ["left_area", "scroll_band", "right_area"]

[left_area]
area_type = "Fixed"
width = 200
plugins = [
    { id = "clock_widget_time" },
    { id = "games_button" }
]

[scroll_band]
area_type = "Scroll"
plugins = [
    { id = "firefox_nightly" },
    { id = "gnome_calculator" }
]

[right_area]
area_type = "Fixed"
width = 150
plugins = [
    { id = "battery" }
]
```

### Button Configuration

```toml
[games_button]
text = "Games"
icon = "applications-games"
click_topic = "area.open"
click_payload = { area_id = "games_area" }
longpress_topic = "area.close"
longpress_payload = { area_id = "games_area" }
```

### Games Submenu Configuration

```toml
[games_area]
area_type = "Scroll"
transition = "SlideLeft"
auto_close = true
close_on_escape = true
plugins = [
    { id = "gnome_2048" },
    { id = "gnome_chess" }
]
```

### Plugin Configurations

```toml
[gnome_2048]
desktop_file_path = "/usr/share/applications/org.gnome.TwentyFortyEight.desktop"

[gnome_chess]
desktop_file_path = "/usr/share/applications/org.gnome.Chess.desktop"
```

## Styling

### CSS Classes

The button widget supports custom CSS classes for styling:

```css
.menu-button {
    padding: 8px 16px;
    border-radius: 4px;
}

.primary {
    background-color: @theme_selected_bg_color;
    color: @theme_selected_fg_color;
}
```

## Future Enhancements

### Button Widget Features

- Support for keyboard shortcuts
- Button state management (active, disabled)
- Animation on button press
- Icon-only mode
- Custom button shapes
- Button groups with radio behavior

### Advanced Area Message System

- **area.reorder** - Reorder areas in the layout
- **area.resize** - Resize an area dynamically
- **area.move** - Move an area to a different position
- **area.show** - Show a hidden area
- **area.hide** - Hide an area
- **area.toggle** - Toggle area visibility
- **area.focus** - Focus on a specific area
- **area.stack.push** - Push area onto stack for nested sub-menus
- **area.stack.pop** - Pop area from stack
- **area.stack.clear** - Clear the area stack
- **area.plugin.add** - Add a plugin to an area
- **area.plugin.remove** - Remove a plugin from an area
- **area.plugin.reorder** - Reorder plugins within an area
- **area.config.update** - Update area configuration dynamically
- **area.transition.set** - Change transition type
- **area.animation.enable/disable** - Control area animations

## Implementation Strategy

### Phase 1: Message System Extension (Backend)

**1. Extend message system to handle area.open/area.close**

- Add message topic constants for area operations
- Extend message payload structure to include area_id
- Ensure message serialization/deserialization works

**2. Implement area message handling in application.rs**

- Add message handler for `area.open` topic
- Add message handler for `area.close` topic
- Integrate with AreaManager for dynamic area operations
- Handle error cases gracefully

**3. Add ButtonConfig to config deserialization**

- Extend SwipeLauncherConfig to include button configurations
- Add ButtonConfig struct with all required fields
- Implement deserialization from TOML
- Add validation for required fields

### Phase 2: Button Widget Plugin (Frontend)

**4. Create button widget plugin crate structure**

- Create `plugins/button` directory
- Set up Cargo.toml with required dependencies
- Create basic plugin structure with widget_plugin! macro
- Implement PluginMetaGetter, MessageHandler, MessageBroadcaster traits

**5. Implement ButtonWidget struct with GTK button**

- Create ButtonWidget struct with GTK button and config
- Implement build_widget() method
- Add icon and text rendering
- Apply CSS classes for styling

**6. Implement click and long-press handlers**

- Add click handler using gtk4::Button::connect_clicked
- Add long-press handler using gtk4::GestureLongPress
- Configure gesture threshold and timing
- Handle both click and long-press events

**7. Implement message sending functionality**

- Integrate message sender from plugin context
- Serialize payload to JSON
- Send messages through FFI envelope
- Handle message sending errors

**8. Register button widget with plugin system**

- Implement widget_plugin! macro
- Ensure proper FFI bindings
- Test plugin loading and registration

### Phase 3: Integration & Testing

**9. Test button widget with games submenu**

- Load button widget from config
- Test single-click to open games area
- Test long-press to close games area
- Verify transition animations work correctly
- Test click-outside and escape key handling

### Rationale

- **Phase 1 first**: The message system must be functional before the button can use it
- **Phase 2 then**: Button widget can only be implemented when infrastructure is ready
- **Phase 3 last**: Integration and testing at the end to validate the complete flow

This order minimizes dependencies and allows step-by-step validation of each component.
