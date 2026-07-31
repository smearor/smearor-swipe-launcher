# Concept: MacroPad Animations and Background

This document describes the concept for **frame-based animations** on MacroPad devices (area-change transitions, button-state-change transitions) and **area
backgrounds** that show through transparent button regions. Both features build on the existing headless rendering pipeline (`GraphicRenderer` trait,
`render_buttons_to_device()` in `application.rs`) and the `MacroPadCommand::SetButtonImage` protocol.

The target frame rate is **20 fps** (50 ms per frame), which is achievable on current hardware given the small pixel dimensions (72×72 px or 90×90 px per key).

---

## 1. Problem Statement

### 1.1 Current State

The current MacroPad rendering pipeline is **static**:

- `render_buttons_to_device()` in `application.rs` renders each plugin once via `render_graphic(width, height)` and sends a single `SetButtonImage` command per
  button.
- When an area changes, all buttons are re-rendered once — no transition animation.
- When a button's state changes (e.g. toggle on/off), the button is re-rendered once — no transition animation.
- There is no concept of an area background. Each button fills its entire pixel buffer with a solid background color (`BG_COLOR` or `BG_COLOR_ACTIVE` in
  `plugins/button/src/graphic.rs`). Buttons are opaque rectangles with no transparency.

### 1.2 What Is Missing

- **Area-change animations**: When the visible area changes (e.g. navigating into a sub-menu), the button images should animate from the old content to the new
  content — not snap instantly.
- **Button-state-change animations**: When a button's internal state changes (e.g. active/inactive toggle), the button image should animate the transition
  rather than swapping instantly.
- **Area backgrounds**: An area should be able to define a background image or gradient that is visible wherever buttons are transparent. The physical gap
  between buttons on the device means the background must account for button spacing.

---

## 2. Goals

- Support **20 fps** frame-based animations on MacroPad devices.
- Provide **area-change transition animations**: Flip Card (with staggered variant), Slide In/Out, Zoom In/Out.
- Provide **button-state-change transition animations**: Flip Card.
- Support **area backgrounds** that show through transparent button regions.
- Account for **button spacing** (physical gaps between keys on the device).
- Optionally support **animated backgrounds** (e.g. looping gradient, subtle motion).
- Keep the animation engine **headless** — no GTK dependency. All rendering is pixel-buffer based via `GraphicRenderer`.
- Make animations **configurable** per area and per button, with sensible defaults.
- Ensure animations are **non-blocking** — the host event loop and message broker continue processing during animations.

## 3. Non-Goals

- Changing the `GraphicRenderer` trait signature (already returns `FfiGraphic`).
- Changing the `MacroPadCommand` protocol (already supports `SetButtonImage`).
- Supporting animations on GTK or Web instances (GTK has its own `LayoutTransition`; Web uses CSS/JS).
- Supporting per-pixel alpha on devices that do not support it (e.g. Stream Deck uses JPEG/BMP which may not preserve alpha — the service handles format
  conversion).
- Frame rates above 20 fps (hardware USB bandwidth and display refresh rate limitations).

---

## 4. Architecture

### 4.1 Animation Engine Overview

A new **MacroPad Animation Engine** runs inside the host (`smearor-swipe-launcher`) and manages frame-based animations for headless instances. It is separate
from the GTK `LayoutTransition` (which uses `glib::timeout_add_local`) because headless instances have no GTK main loop.

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Host (application.rs)                          │
│                                                                      │
│  ┌──────────────────────┐    ┌───────────────────────────────────┐   │
│  │  AreaManager          │    │  MacroPadAnimationEngine          │   │
│  │  (headless instance)  │    │                                   │   │
│  │                       │    │  - AnimationState per instance    │   │
│  │  visible_area change  │───>│  - Active animations queue        │   │
│  │  button state change  │───>│  - Frame timer (50 ms / 20 fps)   │   │
│  └──────────────────────┘    │                                   │   │
│                              │  For each frame:                   │   │
│                              │  1. Compute interpolated frame     │   │
│                              │  2. render_graphic() or composite  │   │
│                              │  3. Send SetButtonImage to service │   │
│                              └───────────────┬───────────────────┘   │
│                                              │                       │
│  ┌───────────────────────────────────────────▼───────────────────┐   │
│  │              Message Broker (FfiEnvelope)                      │   │
│  │  service.macropad.command → SetButtonImage per frame           │   │
│  └───────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
```

### 4.2 Frame Timer

The animation engine uses a `tokio::time::interval` with a 50 ms period (20 fps). Each tick, the engine checks for active animations and renders the next frame
for each animated button.

```rust
/// Animation frame rate: 20 frames per second.
const ANIMATION_FPS: u32 = 20;
/// Duration of one animation frame in milliseconds.
const FRAME_DURATION_MS: u64 = 50;
```

The engine spawns a single async task per headless instance via `PluginExecutor`. This task loops on `interval.tick().await` and processes all active animations
for that instance. This follows the project rule: services and headless animation loops use `tokio::sync::mpsc` and async tasks, not `timeout_add_local`
polling.

### 4.3 Animation Lifecycle

```
1. Trigger (area change or button state change)
   → Host calls MacroPadAnimationEngine::start_animation(instance_id, animation)
   → Engine enqueues animation with start time, duration, type, affected buttons

2. Frame loop (every 50 ms)
   → For each active animation:
     a. Compute progress (0.0 - 1.0) based on elapsed time
     b. Apply easing function
     c. Render frame: call render_graphic() on source and target, then composite
     d. Send SetButtonImage for each affected button index
   → Remove completed animations

3. Completion
   → Final frame renders the target state exactly
   → Animation is removed from the queue
   → Button displays the stable target image
```

### 4.4 Rendering During Animation

During an animation, the engine needs both the **source frame** (old button image) and the **target frame** (new button image). The compositing step depends on
the animation type:

| Animation Type | Compositing Method                                                                                         |
|----------------|------------------------------------------------------------------------------------------------------------|
| Flip Card      | Perspective transform: show source frame shrinking horizontally, then target frame expanding from the back |
| Slide In       | Target frame slides in from edge; source frame slides out opposite direction                               |
| Slide Out      | Source frame slides out to edge; target frame slides in from opposite edge                                 |
| Zoom In        | Target frame scales from 0% to 100%; source frame fades out                                                |
| Zoom Out       | Source frame scales from 100% to 0%; target frame fades in                                                 |

The source and target frames are pre-rendered once at the start of the animation via `render_graphic()`. The engine then composites intermediate frames from
these two cached images — it does **not** call `render_graphic()` on every frame. This keeps CPU usage low.

---

## 5. Area-Change Animations

### 5.1 Overview

When the visible area changes (e.g. `area.open` or `area.close` triggers a new area becoming visible), all buttons on the device transition from the old area's
button images to the new area's button images.

### 5.2 Animation Types

#### 5.2.1 Flip Card

Each button image rotates around a vertical (or horizontal) axis. At the midpoint (90°), the image is edge-on (invisible). The back side then rotates into view
showing the new button image.

```
Frame 0 (0°)     Frame 5 (45°)    Frame 10 (90°)   Frame 15 (135°)  Frame 20 (180°)
┌────────┐       ┌──┐             │                ┌──┐             ┌────────┐
│ SOURCE │  →    │S│      →       │      →         │T│      →       │ TARGET │
│        │       └──┘             │                └──┘             │        │
└────────┘                                            └────────┘
```

**Implementation**: The horizontal scale factor is `cos(progress * π)`. For the first half (0° → 90°), the source frame is drawn scaled horizontally by
`cos(angle)`. For the second half (90° → 180°), the target frame is drawn scaled horizontally by `|cos(angle)|`.

**Axis variant**: The flip axis can be vertical (horizontal scale changes) or horizontal (vertical scale changes). Configurable via `flip_axis`.

#### 5.2.2 Flip Card Staggered

Same as Flip Card, but each button starts its animation with a **delay** proportional to its index. This creates a wave/ripple effect across the button grid.

```
Button 0:  ████░░░░░░░░░░░░░░░░  (starts at 0 ms)
Button 1:  ░░████░░░░░░░░░░░░░░  (starts at 50 ms)
Button 2:  ░░░░████░░░░░░░░░░░░  (starts at 100 ms)
Button 3:  ░░░░░░████░░░░░░░░░░  (starts at 150 ms)
...
```

**Stagger delay**: Configurable, default 50 ms per button index (one frame). The total animation duration is
`base_duration + (button_count - 1) * stagger_delay`.

#### 5.2.3 Slide In / Slide Out

The new button images slide in from one edge of the button grid while the old images slide out the opposite edge.

```
Slide In (from left):

Frame 0:   [OLD][OLD][OLD][OLD][OLD]
Frame 5:   [D][OLD][OLD][OLD][OLD]   (partial new on left)
Frame 10:  [NEW][D][OLD][OLD][OLD]
Frame 15:  [NEW][NEW][D][OLD][OLD]
Frame 20:  [NEW][NEW][NEW][NEW][NEW]

D = transition zone (blend or gap)
```

**Implementation**: Each button's frame is a horizontal crop of the source/target images. At progress `p`, the source image is shifted right by
`p * button_width` and the target image is shifted left by `(1-p) * button_width`. The two halves are composited into a single button image.

**Direction**: Configurable — `left`, `right`, `up`, `down`.

#### 5.2.4 Zoom In / Zoom Out

**Zoom In**: The new button image scales from 0% to 100% centered on the button. The old image fades out simultaneously.

**Zoom Out**: The old button image scales from 100% to 0% centered on the button. The new image fades in simultaneously.

```
Zoom In:

Frame 0:   ┌────────┐
           │  OLD   │
           └────────┘

Frame 10:  ┌────────┐
           │  OLD   │   (fading out)
           │ ┌──┐   │   (NEW scaling in, centered)
           │ │N │   │
           │ └──┘   │
           └────────┘

Frame 20:  ┌────────┐
           │  NEW   │
           └────────┘
```

**Implementation**: Scale the target image by `progress` and composite it centered over the source image. Apply alpha blending: source alpha = `1.0 - progress`,
target alpha = `progress`.

### 5.3 Configuration

Area-change animations are configured per area in the TOML config:

```toml
[scroll_band]
area_type = "scroll"
# Animation when this area becomes visible
enter_animation = "flip_card"
# Animation when this area is closed
exit_animation = "flip_card"
# Animation duration in milliseconds
animation_duration_ms = 500
# Flip card axis: "vertical" or "horizontal"
flip_axis = "vertical"
# Staggered flip: delay per button in milliseconds (0 = no stagger)
stagger_delay_ms = 50
```

Supported `enter_animation` / `exit_animation` values:

| Value                 | Description                             |
|-----------------------|-----------------------------------------|
| `none`                | No animation, instant swap (default)    |
| `flip_card`           | Flip card animation                     |
| `flip_card_staggered` | Flip card with per-button stagger delay |
| `slide_in_left`       | Slide in from left edge                 |
| `slide_in_right`      | Slide in from right edge                |
| `slide_in_up`         | Slide in from top edge                  |
| `slide_in_down`       | Slide in from bottom edge               |
| `slide_out_left`      | Slide out to left edge                  |
| `slide_out_right`     | Slide out to right edge                 |
| `slide_out_up`        | Slide out to top edge                   |
| `slide_out_down`      | Slide out to bottom edge                |
| `zoom_in`             | Zoom in from center                     |
| `zoom_out`            | Zoom out from center                    |

### 5.4 Trigger Flow

```
1. User presses button that triggers area.open (or area.close)
2. Host processes the action, area manager switches visible area
3. Host calls MacroPadAnimationEngine::start_area_transition(
     instance_id,
     old_plugin_ids,   // buttons currently displayed
     new_plugin_ids,   // buttons to display after transition
     enter_animation,  // from area config
     duration_ms,
     stagger_delay_ms,
   )
4. Engine pre-renders source frames (old plugins) and target frames (new plugins)
5. Engine runs frame loop at 20 fps, compositing and sending SetButtonImage per frame
6. On completion, engine sends final target frames and marks animation complete
7. Host resumes normal static rendering for subsequent state changes
```

---

## 6. Button-State-Change Animations

### 6.1 Overview

When a button's internal state changes (e.g. a toggle button switches from inactive to active, or a `state_topic` message changes the button's icon/label), the
button image should animate the transition.

### 6.2 Animation Type: Flip Card

The button image flips around a vertical axis. The front side shows the old state, the back side shows the new state.

```
Frame 0 (0°)     Frame 10 (90°)   Frame 20 (180°)
┌────────┐        │               ┌────────┐
│  OLD   │   →    │      →        │  NEW   │
│ STATE  │        │               │ STATE  │
└────────┘                        └────────┘
```

This is the same flip card mechanism as the area-change animation, but applied to a single button and triggered by a state change rather than an area change.

### 6.3 Trigger Flow

```
1. Button widget receives state update via state_topic message
2. Widget updates internal_state and broadcasts WidgetUpdateMessage
3. Host receives widget.update, checks if button has state_change_animation configured
4. If yes:
   a. Host pre-renders source frame (old state) — already cached from last render
   b. Host pre-renders target frame (new state) via render_graphic()
   c. Host calls MacroPadAnimationEngine::start_button_transition(
        instance_id,
        button_index,
        source_frame,
        target_frame,
        animation_type,
        duration_ms,
      )
   d. Engine runs frame loop for this single button
5. If no animation configured:
   a. Host renders new frame and sends SetButtonImage directly (current behavior)
```

### 6.4 Configuration

Button-state-change animations are configured per button in the TOML config:

```toml
[toggle_button]
defaults = "menu_button"
text = "Toggle"
icon = "nf-md-toggle_switch"
state_change_animation = "flip_card"
state_change_animation_duration_ms = 300
flip_axis = "vertical"
```

Supported `state_change_animation` values:

| Value       | Description                          |
|-------------|--------------------------------------|
| `none`      | No animation, instant swap (default) |
| `flip_card` | Flip card animation                  |

Only `flip_card` is planned for button-state-change animations. Slide and zoom animations are area-change-only because they rely on multi-button grid effects.

---

## 7. MacroPad Background

### 7.1 Overview

An area can define a **background** that is drawn behind the buttons. Wherever a button's pixel buffer has transparency (alpha < 255), the background shows
through. This creates a unified visual surface across the device, with buttons appearing as cut-outs or overlays on top of the background.

### 7.2 Button Spacing

MacroPad devices have physical gaps between keys. On a Stream Deck MK.2, each key is 72×72 px but the physical key is approximately 16.1 × 16.1 mm with a small
gap between adjacent keys. The background must account for this gap:

- The **full area background** is rendered as a single large image spanning the entire button grid (including gaps).
- Each button receives a **crop** of the background that corresponds to its physical position on the grid.
- The button's own pixel buffer is composited **over** the background crop.
- Where the button is transparent, the background crop is visible.

### 7.3 Background Rendering Pipeline

```
1. Area config defines a background (image path, gradient, or color)
2. Host renders the full background image at device resolution:
   total_width = key_width * columns + gap_width * (columns - 1)
   total_height = key_height * rows + gap_height * (rows - 1)
3. For each button at position (row, col):
   a. Crop the background region for this button:
      crop_x = col * (key_width + gap_width)
      crop_y = row * (key_height + gap_height)
      crop = background.sub_image(crop_x, crop_y, key_width, key_height)
   b. Render the button via render_graphic(key_width, key_height)
   c. Composite button over background crop:
      for each pixel:
        if button_alpha > 0:
          result = blend(button_pixel, background_pixel, button_alpha)
        else:
          result = background_pixel
   d. Send composited image as SetButtonImage
```

### 7.4 Background Types

#### 7.4.1 Solid Color

A single RGBA color filling the entire background.

```toml
[scroll_band.background]
type = "color"
color = "#1a1a2e"
```

#### 7.4.2 Image

A static image file (PNG, JPEG, BMP) scaled to fit the full grid dimensions.

```toml
[scroll_band.background]
type = "image"
path = "resources/backgrounds/waves.png"
# How to handle aspect ratio mismatch:
# "stretch" — fill exactly (may distort)
# "cover"   — fill and crop (may cut off edges)
# "contain" — fit entirely (may leave bars)
fit = "cover"
```

#### 7.4.3 Gradient

A linear or radial gradient defined by color stops.

```toml
[scroll_band.background]
type = "gradient"
gradient_type = "linear"
angle = 45  # degrees, 0 = top-to-bottom, 90 = left-to-right
stops = [
    { position = 0.0, color = "#1a1a2e" },
    { position = 1.0, color = "#0f3460" },
]
```

#### 7.4.4 Animated Background

A background that changes over time. The animation engine renders a new background frame at the configured fps and re-composites all buttons.

```toml
[scroll_band.background]
type = "animated"
fps = 10  # background animation frame rate (can be lower than button animation fps)
source = "gradient_cycle"  # built-in animated gradient
# Or: source = "gif", path = "resources/backgrounds/loop.gif"

# Gradient cycle parameters
colors = ["#1a1a2e", "#0f3460", "#16213e"]
cycle_duration_ms = 5000  # full cycle duration
```

**Built-in animated sources**:

| Source           | Description                                                                      |
|------------------|----------------------------------------------------------------------------------|
| `gradient_cycle` | Animated gradient that cycles through configured colors over `cycle_duration_ms` |
| `gif`            | Animated GIF file, looped continuously                                           |
| `rain`           | Simple rain/particle effect (procedural)                                         |
| `plasma`         | Procedural plasma effect (sinusoidal color mapping)                              |

Animated backgrounds use the same frame timer as button animations but at a potentially lower frame rate (configurable, default 10 fps) to reduce CPU load. The
background frame is rendered independently of button state changes.

### 7.5 Button Transparency

For the background to show through, buttons must support **transparent regions** in their pixel buffers. Currently, `fill_background()` in
`plugins/button/src/graphic.rs` fills the entire buffer with an opaque color (`[30, 30, 30, 255]`).

To support backgrounds, the button rendering must be modified:

1. **New config field**: `transparent_background: bool` (default: `false`). When `true`, the button does not fill the background — transparent regions show the
   area background.
2. **Rounded corners**: Optional `corner_radius: u32` config field. When set, the button background is drawn as a rounded rectangle, and the corner regions are
   transparent, showing the area background.
3. **Compositing**: The host composites the button image over the background crop using alpha blending. The button's alpha channel determines how much of the
   background is visible.

```toml
[toggle_button]
defaults = "menu_button"
text = "Toggle"
icon = "nf-md-toggle_switch"
transparent_background = true
corner_radius = 8  # rounded corners, 0 = square
```

### 7.6 Background Compositing in the Host

The background compositing happens in `render_buttons_to_device()` (or a new `render_buttons_with_background()` method) in `application.rs`:

```rust
/// Render all visible area plugins to button images, composited over the area background,
/// and send them to the MacroPad device.
pub fn render_buttons_to_device_with_background(
    &self,
    instance_id: &str,
    background: &AreaBackground,
    button_spacing: &ButtonSpacing,
) {
    // 1. Render full background image at grid resolution
    let full_bg = background.render(grid_width, grid_height);

    // 2. For each button:
    for (index, plugin_id) in plugin_ids.iter().enumerate() {
        let (row, col) = index_to_row_col(index, columns);

        // Crop background for this button position
        let bg_crop = crop_background(&full_bg, row, col, button_spacing);

        // Render button graphic
        let button_graphic = plugin.render_graphic(key_width, key_height);

        // Composite button over background crop
        let composited = composite_over(&bg_crop, &button_graphic);

        // Send to device
        send_set_button_image(device_id, index, composited);
    }
}
```

### 7.7 Button Spacing Configuration

Button spacing is device-specific and determined by the physical key layout. It is communicated in the `MacroPadConnectionStatus` message or configured per
instance:

```toml
[launcher]
instance_id = "macropad_1"

[macropad]
# Button grid layout
columns = 5
rows = 3
# Gap between buttons in pixels (at device resolution)
gap_x = 4
gap_y = 4
```

If not configured, the host defaults to zero gap (buttons are adjacent, no visible background between them).

---

## 8. Model Extensions

### 8.1 Animation Types (`model/macropad`)

New types added to `model/macropad`:

```rust
/// Animation type for MacroPad transitions.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum MacroPadAnimation {
    /// No animation, instant swap.
    #[default]
    None,
    /// Flip card animation (single button or all buttons simultaneously).
    FlipCard {
        /// Flip axis: "vertical" or "horizontal".
        axis: stabby::string::String,
    },
    /// Flip card with staggered per-button delay.
    FlipCardStaggered {
        /// Flip axis: "vertical" or "horizontal".
        axis: stabby::string::String,
        /// Delay per button index in milliseconds.
        stagger_delay_ms: u32,
    },
    /// Slide animation.
    Slide {
        /// Direction: "left", "right", "up", "down".
        direction: stabby::string::String,
        /// True for slide-in, false for slide-out.
        slide_in: bool,
    },
    /// Zoom animation.
    Zoom {
        /// True for zoom-in, false for zoom-out.
        zoom_in: bool,
    },
}

/// Background type for an area.
#[repr(u8)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub enum AreaBackgroundType {
    /// No background (buttons are fully opaque).
    #[default]
    None,
    /// Solid color background.
    Color,
    /// Static image background.
    Image,
    /// Gradient background.
    Gradient,
    /// Animated background.
    Animated,
}
```

### 8.2 Background Config (`model/area`)

Extended `AreaConfig` with optional background configuration:

```rust
/// Background configuration for an area.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AreaBackgroundConfig {
    /// Type of background.
    #[serde(default)]
    pub background_type: AreaBackgroundType,
    /// Solid color (for Color type).
    #[serde(default)]
    pub color: Option<String>,
    /// Image path (for Image type).
    #[serde(default)]
    pub path: Option<String>,
    /// Image fit mode (for Image type).
    #[serde(default)]
    pub fit: Option<String>,
    /// Gradient type (for Gradient type).
    #[serde(default)]
    pub gradient_type: Option<String>,
    /// Gradient angle in degrees (for Gradient type).
    #[serde(default)]
    pub angle: Option<f32>,
    /// Gradient color stops (for Gradient type).
    #[serde(default)]
    pub stops: Vec<GradientStop>,
    /// Animated source name (for Animated type).
    #[serde(default)]
    pub source: Option<String>,
    /// Animation fps (for Animated type).
    #[serde(default)]
    pub fps: Option<u32>,
    /// Cycle duration in milliseconds (for Animated type).
    #[serde(default)]
    pub cycle_duration_ms: Option<u32>,
    /// Colors for animated gradient cycle.
    #[serde(default)]
    pub colors: Vec<String>,
}

/// A single color stop in a gradient.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GradientStop {
    /// Position in the gradient (0.0 - 1.0).
    pub position: f32,
    /// Color as hex string (e.g. "#1a1a2e").
    pub color: String,
}
```

### 8.3 Button Grid Layout (`model/macropad`)

```rust
/// Physical button grid layout for a MacroPad device.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[stabby::stabby]
pub struct ButtonGridLayout {
    /// Number of columns in the button grid.
    pub columns: u8,
    /// Number of rows in the button grid.
    pub rows: u8,
    /// Horizontal gap between buttons in pixels.
    pub gap_x: u32,
    /// Vertical gap between buttons in pixels.
    pub gap_y: u32,
}
```

---

## 9. Animation Engine Implementation

### 9.1 MacroPadAnimationEngine

A new module in `smearor-swipe-launcher/src/area/animation_engine.rs` (or `smearor-swipe-launcher/src/macropad/animation_engine.rs`):

```rust
/// Manages frame-based animations for headless MacroPad instances.
pub struct MacroPadAnimationEngine {
    /// Active animations keyed by instance ID.
    animations: Arc<Mutex<HashMap<String, Vec<ActiveAnimation>>>>,
    /// Broker sender for dispatching SetButtonImage commands.
    broker_sender: Sender<FfiEnvelope>,
}

/// A single active animation.
struct ActiveAnimation {
    /// Animation type and parameters.
    animation: MacroPadAnimation,
    /// Start time (monotonic).
    start_time: Instant,
    /// Total duration in milliseconds.
    duration_ms: u32,
    /// Affected button indices.
    button_indices: Vec<u8>,
    /// Pre-rendered source frames (one per button index).
    source_frames: Vec<FfiGraphic>,
    /// Pre-rendered target frames (one per button index).
    target_frames: Vec<FfiGraphic>,
    /// Device ID for sending commands.
    device_id: String,
    /// Driver instance ID for routing.
    driver_instance_id: String,
    /// Sender instance ID.
    sender_instance_id: String,
}
```

### 9.2 Frame Compositing Functions

```rust
/// Composite a flip card frame from source and target images.
fn composite_flip_card(
    source: &FfiGraphic,
    target: &FfiGraphic,
    progress: f32,
    axis: FlipAxis,
) -> Vec<u8>

/// Composite a slide frame from source and target images.
fn composite_slide(
    source: &FfiGraphic,
    target: &FfiGraphic,
    progress: f32,
    direction: SlideDirection,
    slide_in: bool,
) -> Vec<u8>

/// Composite a zoom frame from source and target images.
fn composite_zoom(
    source: &FfiGraphic,
    target: &FfiGraphic,
    progress: f32,
    zoom_in: bool,
) -> Vec<u8>

/// Composite a button image over a background crop using alpha blending.
fn composite_over(
    background: &[u8],
    button: &[u8],
    width: u32,
    height: u32,
) -> Vec<u8>
```

### 9.3 Easing Functions

```rust
/// Ease-in-out cubic for smooth acceleration and deceleration.
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - ((-2.0 * t + 2.0).powi(3)) / 2.0
    }
}

/// Ease-out cubic for smooth deceleration (matches existing LayoutTransition).
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
```

### 9.4 Background Renderer

```rust
/// Render an area background to a full-grid image.
pub struct AreaBackgroundRenderer {
    /// Cached full-grid background image.
    cached_image: Option<RgbaImage>,
    /// Last rendered frame index (for animated backgrounds).
    frame_index: u32,
}

impl AreaBackgroundRenderer {
    /// Render the background at the given grid dimensions.
    pub fn render(&mut self, config: &AreaBackgroundConfig, grid_width: u32, grid_height: u32) -> RgbaImage;

    /// Render a single animated frame.
    pub fn render_animated_frame(&mut self, config: &AreaBackgroundConfig, grid_width: u32, grid_height: u32, frame: u32) -> RgbaImage;

    /// Crop the background for a specific button position.
    pub fn crop_for_button(&self, row: u8, col: u8, key_width: u32, key_height: u32, spacing: &ButtonGridLayout) -> RgbaImage;
}
```

---

## 10. Host Integration

### 10.1 Area-Change Animation Integration

In `application.rs`, when the visible area changes for a headless instance:

```rust
/// Handle area change for a headless instance with animation.
pub fn handle_area_change_animated(&self, instance_id: &str, old_plugin_ids: Vec<String>, new_plugin_ids: Vec<String>) {
    let area_config = self.get_area_config(instance_id, &new_area_id);
    let animation = area_config.enter_animation.clone();
    let duration_ms = area_config.animation_duration_ms;
    let stagger_delay_ms = area_config.stagger_delay_ms;

    if animation == MacroPadAnimation::None {
        // Current behavior: render once, no animation
        self.render_buttons_to_device(instance_id);
    } else {
        // Start animated transition
        self.animation_engine.start_area_transition(
            instance_id,
            old_plugin_ids,
            new_plugin_ids,
            animation,
            duration_ms,
            stagger_delay_ms,
        );
    }
}
```

### 10.2 Button-State-Change Animation Integration

When a `WidgetUpdateMessage` is received for a headless instance:

```rust
/// Handle widget update for a headless instance with optional animation.
pub fn handle_widget_update_animated(&self, instance_id: &str, plugin_id: &str) {
    let button_config = self.get_button_config(instance_id, plugin_id);
    let animation = button_config.state_change_animation;
    let duration_ms = button_config.state_change_animation_duration_ms;

    if animation == MacroPadAnimation::None {
        // Current behavior: render once, no animation
        self.render_single_button_to_device(instance_id, plugin_id);
    } else {
        // Start button-state-change animation
        let button_index = self.get_button_index(instance_id, plugin_id);
        let source_frame = self.get_cached_button_image(instance_id, button_index);
        let target_frame = self.render_button_graphic(instance_id, plugin_id);

        self.animation_engine.start_button_transition(
            instance_id,
            button_index,
            source_frame,
            target_frame,
            animation,
            duration_ms,
        );
    }
}
```

### 10.3 Background Integration

In `render_buttons_to_device()` (or a new `render_buttons_to_device_with_background()`):

```rust
/// Render all visible area plugins to button images, composited over the area background.
pub fn render_buttons_to_device(&self, instance_id: &str) {
    let area_config = self.get_visible_area_config(instance_id);
    let background_config = &area_config.background;
    let grid_layout = self.get_button_grid_layout(instance_id);

    if background_config.background_type == AreaBackgroundType::None {
        // Current behavior: render buttons without background
        self.render_buttons_only(instance_id);
    } else {
        // Render with background compositing
        let mut bg_renderer = AreaBackgroundRenderer::new();
        let full_bg = bg_renderer.render(background_config, grid_width, grid_height);

        for (index, plugin_id) in plugin_ids.iter().enumerate() {
            let (row, col) = index_to_row_col(index, grid_layout.columns);
            let bg_crop = bg_renderer.crop_for_button(row, col, key_width, key_height, &grid_layout);
            let button_graphic = plugin.render_graphic(key_width, key_height);
            let composited = composite_over(&bg_crop, &button_graphic, key_width, key_height);
            send_set_button_image(device_id, index, composited);
        }
    }
}
```

### 10.4 Animated Background Loop

For animated backgrounds, a separate async task is spawned per instance:

```rust
/// Spawn a background animation loop for a headless instance.
pub fn spawn_background_animation_loop(&self, instance_id: &str, fps: u32) {
    let interval_duration = Duration::from_millis(1000 / fps as u64);
    let instance_id = instance_id.to_string();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(interval_duration);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            // Re-render background and composite all buttons
            self.render_buttons_to_device_with_animated_background(&instance_id);
        }
    });
}
```

This task runs alongside the button animation engine. When both are active, the background loop provides the background frame and the button animation engine
composites its animated buttons over the current background frame.

---

## 11. Configuration Examples

### 11.1 Area with Flip Card Animation and Gradient Background

```toml
areas = ["main"]

[main]
area_type = "fixed"
enter_animation = "flip_card_staggered"
exit_animation = "flip_card_staggered"
animation_duration_ms = 500
flip_axis = "vertical"
stagger_delay_ms = 50

[main.background]
type = "gradient"
gradient_type = "linear"
angle = 45
stops = [
    { position = 0.0, color = "#1a1a2e" },
    { position = 1.0, color = "#0f3460" },
]

[[main.plugins]]
id = "button_apps"
path = "target/release/libsmearor_button_widget.so"
text = "Apps"
icon = "nf-md-apps"
transparent_background = true
corner_radius = 8
click_topic = "area.app_launcher.open"
click_instance = "macropad_1"

[[main.plugins]]
id = "button_weather"
path = "target/release/libsmearor_button_widget.so"
text = "Weather"
icon = "nf-md-weather-partly-cloudy"
transparent_background = true
corner_radius = 8
click_topic = "area.weather.open"
click_instance = "macropad_1"
```

### 11.2 Button with State-Change Flip Card Animation

```toml
[toggle_button]
defaults = "menu_button"
text = "Floating"
icon = "nf-md-window_restore"
state_topic = "service.hyprland.workspace.status"
state_icon = "{floating?nf-md-window_maximize:nf-md-window_restore}"
state_change_animation = "flip_card"
state_change_animation_duration_ms = 300
flip_axis = "vertical"
transparent_background = true
corner_radius = 8
click_topic = "service.hyprland.dispatch"
click_payload = { kind = "ToggleFloating", identifier = { kind = "Active" } }
```

### 11.3 Area with Animated Background

```toml
[main]
area_type = "fixed"
enter_animation = "slide_in_left"
animation_duration_ms = 400

[main.background]
type = "animated"
source = "gradient_cycle"
fps = 10
cycle_duration_ms = 5000
colors = ["#1a1a2e", "#0f3460", "#16213e"]

[macropad]
columns = 5
rows = 3
gap_x = 4
gap_y = 4
```

### 11.4 Button Grid Layout

```toml
[launcher]
instance_id = "macropad_1"

[macropad]
columns = 5
rows = 3
gap_x = 4
gap_y = 4
```

---

## 12. Implementation Phases

### Phase 1: Animation Engine Core

**Order**: First. All animation phases depend on this.

**Changes**:

- Create `smearor-swipe-launcher/src/macropad/animation_engine.rs` with `MacroPadAnimationEngine`.
- Implement frame timer via `tokio::time::interval` at 20 fps.
- Implement `ActiveAnimation` struct with source/target frame caching.
- Implement easing functions (`ease_in_out_cubic`, `ease_out_cubic`).
- Add `MacroPadAnimation` enum to `model/macropad`.
- Add to workspace `Cargo.toml`.

**Exit Criteria**: Engine compiles, frame timer runs at 20 fps, animations can be enqueued and processed.

### Phase 2: Area-Change Animations

**Order**: After Phase 1.

**Changes**:

- Implement `composite_flip_card()`, `composite_slide()`, `composite_zoom()` functions.
- Implement staggered flip card with per-button delay.
- Integrate with `handle_area_change` in `application.rs` for headless instances.
- Add `enter_animation`, `exit_animation`, `animation_duration_ms`, `flip_axis`, `stagger_delay_ms` to `AreaConfig`.
- Pre-render source and target frames at animation start.
- Send `SetButtonImage` per frame via broker.

**Exit Criteria**: Area changes on MacroPad devices animate smoothly at 20 fps. All animation types work. Staggered flip creates wave effect.

### Phase 3: Button-State-Change Animations

**Order**: After Phase 2.

**Changes**:

- Implement button-state-change trigger via `WidgetUpdateMessage`.
- Add `state_change_animation`, `state_change_animation_duration_ms` to `ButtonConfig`.
- Cache last rendered button image for use as source frame.
- Integrate with `handle_widget_update` in `application.rs` for headless instances.
- Only `flip_card` animation type for state changes.

**Exit Criteria**: Button state changes animate with flip card on MacroPad devices. Non-animated buttons still work with instant swap.

### Phase 4: Area Background — Static

**Order**: After Phase 2 (needs compositing infrastructure).

**Changes**:

- Add `AreaBackgroundConfig`, `AreaBackgroundType`, `GradientStop` to `model/area`.
- Add `background` field to `AreaConfig`.
- Implement `AreaBackgroundRenderer` with `render()` for color, image, gradient types.
- Implement `composite_over()` alpha blending function.
- Implement `crop_for_button()` with button spacing support.
- Add `ButtonGridLayout` to `model/macropad`.
- Add `transparent_background`, `corner_radius` to `ButtonConfig`.
- Modify `fill_background()` in `plugins/button/src/graphic.rs` to support transparent mode.
- Integrate background compositing in `render_buttons_to_device()`.

**Exit Criteria**: Static backgrounds (color, image, gradient) render correctly. Button transparency shows background through gaps and rounded corners. Button
spacing is respected.

### Phase 5: Area Background — Animated

**Order**: After Phase 4.

**Changes**:

- Implement animated background sources: `gradient_cycle`, `gif`, `rain`, `plasma`.
- Spawn background animation loop via `tokio::spawn` with configurable fps.
- Re-composite all buttons on each background frame.
- Add `fps`, `cycle_duration_ms`, `colors`, `source` to `AreaBackgroundConfig`.
- Handle background animation pause/resume when instance is idle (no visible area change).

**Exit Criteria**: Animated backgrounds loop smoothly. CPU usage is reasonable at 10 fps background animation. Buttons composite correctly over moving
background.

### Phase 6: Polish and Testing

**Order**: After Phase 5.

**Changes**:

- Integration tests: verify animation frame count, duration, and final frame correctness.
- Integration tests: verify background compositing with transparent buttons.
- Config examples: `config-macropad-animations.toml`, `config-macropad-background.toml`.
- Performance benchmarks: measure CPU usage during 20 fps animation loop.
- Debouncing: prevent animation restart if area changes rapidly.
- Fallback: if animation engine fails, fall back to instant swap (current behavior).
- Documentation: update `docs/MACRO_PAD_MODELS.md` with animation and background sections.

**Exit Criteria**: All animations and backgrounds work on physical devices. No frame drops at 20 fps. CPU usage during idle (no animation) is zero.

---

## 13. File Changes Summary

| File                                                         | Change                                                                                                                                 |
|--------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------|
| `model/macropad/src/lib.rs`                                  | Add `MacroPadAnimation`, `ButtonGridLayout` types                                                                                      |
| `model/area/src/lib.rs`                                      | Add `AreaBackgroundConfig`, `AreaBackgroundType`, `GradientStop`                                                                       |
| `model/area/src/area_config.rs`                              | Add `background`, `enter_animation`, `exit_animation`, `animation_duration_ms`, `flip_axis`, `stagger_delay_ms` fields to `AreaConfig` |
| `plugins/button/src/config.rs`                               | Add `transparent_background`, `corner_radius`, `state_change_animation`, `state_change_animation_duration_ms` fields to `ButtonConfig` |
| `plugins/button/src/graphic.rs`                              | Modify `fill_background()` to support transparent mode and rounded corners                                                             |
| `smearor-swipe-launcher/src/macropad/animation_engine.rs`    | **New** — `MacroPadAnimationEngine`, frame compositing, easing functions                                                               |
| `smearor-swipe-launcher/src/macropad/background_renderer.rs` | **New** — `AreaBackgroundRenderer`, background compositing                                                                             |
| `smearor-swipe-launcher/src/macropad/mod.rs`                 | **New** — module declarations                                                                                                          |
| `smearor-swipe-launcher/src/application.rs`                  | Integrate animation engine and background renderer into `render_buttons_to_device()`, `handle_area_change()`, `handle_widget_update()` |
| `smearor-swipe-launcher/src/instance/launcher_instance.rs`   | Add `ButtonGridLayout` to instance metadata                                                                                            |
| `Cargo.toml` (workspace)                                     | No new external crates required                                                                                                        |

---

## 14. Dependencies

### New External Dependencies

None. All required crates (`image`, `ab_glyph`, `imageproc`, `tokio`) are already in the workspace.

### Per-Crate Additions

| Crate                    | Additional Dependencies                                |
|--------------------------|--------------------------------------------------------|
| `model/macropad`         | None (already has `serde`, `stabby`)                   |
| `model/area`             | None (already has `serde`)                             |
| `plugins/button`         | None (already has `image`, `ab_glyph`)                 |
| `smearor-swipe-launcher` | `image` (for background image loading and compositing) |

---

## 15. Performance Considerations

### 15.1 Frame Budget

At 20 fps, each frame has a **50 ms budget**. The following operations must complete within this budget:

| Operation                            | Estimated Time | Notes                                     |
|--------------------------------------|----------------|-------------------------------------------|
| Compositing one button frame         | < 1 ms         | 72×72×4 = 20,736 bytes, simple pixel math |
| Compositing 15 buttons               | < 15 ms        | 15 × 1 ms                                 |
| Sending 15 `SetButtonImage` commands | < 5 ms         | Broker channel send, no I/O               |
| Service writes 15 images to device   | < 20 ms        | USB HID transfer, service-thread          |
| **Total**                            | **< 40 ms**    | Within 50 ms budget                       |

### 15.2 Pre-Rendering Strategy

Source and target frames are pre-rendered **once** at animation start via `render_graphic()`. Intermediate frames are composited from these cached images using
pixel math only — no font rasterization or glyph rendering during the animation. This keeps per-frame CPU cost minimal.

### 15.3 Animated Background CPU Cost

Animated backgrounds at 10 fps require re-compositing all buttons every 100 ms. With 15 buttons at 72×72 px, this is approximately 15 × 1 ms = 15 ms per frame,
well within the 100 ms budget.

### 15.4 Idle Behavior

When no animation is active and the background is static, the animation engine is idle — no frame timer ticks, no CPU usage. The background animation loop only
runs when `background_type = "animated"` is configured.

---

## 16. Risks and Considerations

1. **USB bandwidth**: Sending 15 button images at 20 fps requires sustained USB throughput. Stream Deck uses USB 2.0 which should handle this, but Loupedeck's
   serial protocol may be slower. The service implementation should buffer frames and skip if the device cannot keep up.

2. **Device display refresh**: Some devices may have a lower display refresh rate than 20 fps. The animation engine should be device-aware — if the service
   reports a lower max fps, the engine should reduce the animation frame rate accordingly.

3. **Alpha channel on devices**: Stream Deck uses JPEG or BMP format for button images, which may not preserve alpha. The service handles format conversion. The
   compositing happens in the host (RGBA), and the service converts the final composited image to the device's required format. The background is already
   composited into the button image before sending — the device receives a fully opaque image.

4. **Animation interruption**: If a new area change or state change occurs while an animation is in progress, the engine should cancel the current animation and
   start the new one. The source frame for the new animation is the last rendered frame of the interrupted animation.

5. **Config complexity**: Adding animation and background config fields increases config complexity. Sensible defaults (`none` for animations, `none` for
   background) ensure existing configs work without changes.

6. **Button index to grid position mapping**: The mapping from linear button index to (row, col) grid position depends on the device's button layout. Stream
   Deck MK.2 is 3×5 (3 rows, 5 columns). The `ButtonGridLayout` config allows this to be specified per instance.

7. **Background image caching**: Static background images should be loaded once and cached. The `AreaBackgroundRenderer` caches the full-grid image and only
   re-renders if the config changes or the background is animated.

8. **Corner radius rendering**: Rounded corners are drawn by modifying the alpha channel of the button's background pixels. Pixels outside the rounded rectangle
   are set to alpha 0, allowing the area background to show through. This is a simple geometric operation with no performance impact.

---

## 17. Open Questions

1. **Device max fps**: Should the `MacroPadConnectionStatus` message include a `max_fps` field so the animation engine can adapt to device capabilities?

2. **Background image format**: Should the background renderer support SVG images for scalable backgrounds, or is PNG/BMP sufficient?

3. **Per-button background override**: Should individual buttons be able to override the area background with their own background image?

4. **Animation queue depth**: Should there be a maximum number of concurrent animations per instance to prevent resource exhaustion?

5. **Background during area transition**: Should the background also animate during an area change (e.g. cross-fade between two backgrounds), or should the
   background swap instantly while buttons animate?

6. **Button gap color**: When `gap_x` / `gap_y` is non-zero, should the gap region show the background, or should it be a configurable gap color (e.g. black)?
