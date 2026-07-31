# Concept: MacroPad Span Groups — Shared State & Interactive Widgets

This document describes the concept for **Shared State across Span Group instances** and the implementation of two interactive multi-span widgets: **Timer**
(stopwatch) and **Countdown** (countdown timer). The current Span Group system renders a combined graphic across multiple buttons but each plugin instance is
independent — there is no shared state, and widgets have no interactive logic. This concept introduces a general mechanism for sharing state across instances of
the same `span_group` and applies it to implement the Timer and Countdown widgets with per-button action bindings.

---

## 1. Problem Statement

### 1.1 Current State

Span Groups were introduced in `MACROPAD_ATOMIC_WIDGETS.md` Phase 8. The host groups plugin entries by `span_group`, renders the first member at combined
dimensions, and splits the pixel buffer across buttons. Each plugin instance is a **separate allocation** — the host calls `smearor_plugin_create` once per
`PluginEntry`, producing independent widget structs with no shared data.

The atomic widget action pipeline works as follows:

1. Host detects button press → `dispatch_macropad_action(instance_id, button_index, action)` (`application.rs:513`).
2. Host sends `InvokeToolMessage` with `{"action":"click"}` to the plugin at that button index.
3. The `atomic_widget_impl!` macro's `MessageHandler<InvokeToolMessage>` receives it, parses the action string, and calls `self.dispatch_action(action)`
   (`plugin-api/src/atomic/macro.rs:614-633`).
4. `dispatch_action` reads `click_topic`/`click_payload` from `AtomicWidgetConfig` and broadcasts the message (`plugin-api/src/atomic/config.rs:79-90`).

This means actions are **purely config-driven**: the widget itself has no opportunity to react to a click with internal logic (e.g. starting a timer). It can
only forward the action to a topic.

### 1.2 What Is Missing

- **Shared State**: All instances of a span group need access to a common state (e.g. `TimerState`). Since each instance is a separate `Box<Widget>` allocated
  by the plugin constructor, they do not share Rust memory.
- **Custom Action Handling**: Widgets need to react to actions with internal logic (start/pause/reset a timer) in addition to or instead of forwarding to a
  topic.
- **Per-Button Action Differentiation**: In a 1×3 span group, Button 0 should start the timer, Button 1 should pause it, Button 2 should reset it — all
  operating on the same shared state.
- **Timer Widget**: A stopwatch that counts up from `00:00`, with start/pause/reset controls.
- **Countdown Widget**: A countdown timer that counts down from a configured time, with increment/start/pause controls.

### 1.3 FFI Constraint

Each plugin instance is loaded via `libloading::Library::new()` and constructed via `smearor_plugin_create()`. When the same `.so` is loaded multiple times
(once per span group member), `libloading` returns the **same `Library` handle** (the OS does not reload the library — it increments a reference count). This
means:

- **Static variables are shared** across instances from the same `.so` file. A `static SPAN_STATE: RwLock<HashMap<String, Arc<Mutex<TimerState>>>>` in the
  plugin crate is visible to all instances.
- **Thread-local state is not shared** (each instance runs on the same thread but has independent stack).
- **Heap allocations are not shared** (each `Box::new(widget)` is independent).

The solution uses a **crate-level static registry** keyed by `span_group` name, allowing all instances from the same `.so` to look up and share an
`Arc<Mutex<State>>`.

---

## 2. Goals

- Provide a **general mechanism** for sharing state across span group instances within the same plugin crate.
- Allow widgets to **react to actions with internal logic** (not just config-driven topic forwarding).
- Support **per-button action differentiation** within a span group (Button 0 = start, Button 1 = pause, Button 2 = reset).
- Implement the **Timer Widget** (stopwatch counting up) with start/pause/reset.
- Implement the **Countdown Widget** (counting down) with increment/start/pause.
- Maintain backward compatibility with existing atomic widgets that use purely config-driven actions.
- Keep the existing `atomic_widget_impl!` macro and `AtomicWidgetConfig` system intact.

## 3. Non-Goals

- Sharing state across **different** `.so` files (e.g. audio widget and clock widget sharing state — not needed).
- Supporting span group state on GTK or Web instances (MacroPad only).
- Changing the `WidgetPluginVTable` or FFI boundary.
- Replacing the config-driven action dispatch (it remains the primary mechanism; custom handling is an additional hook).

---

## 4. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│  Plugin Crate (e.g. plugins/clock)                                    │
│                                                                       │
│  static SPAN_STATE_REGISTRY: RwLock<HashMap<String, Arc<Mutex<…>>>>  │
│                                                                       │
│  ┌─────────────────────┐  ┌─────────────────────┐  ┌──────────────┐ │
│  │ Timer Instance 0    │  │ Timer Instance 1    │  │ Timer Inst 2 │ │
│  │ (span_index=0)      │  │ (span_index=1)      │  │ (span_idx=2) │ │
│  │                     │  │                     │  │              │ │
│  │  self.state ────────┼──┤  self.state ────────┼──┤  self.state  │ │
│  │  (Arc<Mutex<State>>)│  │  (Arc<Mutex<State>>)│  │  (same Arc)  │ │
│  └─────────────────────┘  └─────────────────────┘  └──────────────┘ │
│              │                       │                     │        │
│              └───────────────────────┴─────────────────────┘        │
│                              │                                       │
│                    Shared TimerState {                               │
│                        status: Idle|Running|Paused,                  │
│                        elapsed: Duration,                            │
│                        last_tick: Instant,                           │
│                    }                                                 │
└──────────────────────────────────────────────────────────────────────┘

Host (application.rs):
  Button 0 pressed → InvokeToolMessage{action:"click"} → Instance 0
    → Instance 0 looks up span_action mapping for span_index=0
    → Calls shared_state.start()
    → Broadcasts WidgetUpdateMessage (triggers re-render of entire span group)

  Button 1 pressed → InvokeToolMessage{action:"click"} → Instance 1
    → Instance 1 looks up span_action mapping for span_index=1
    → Calls shared_state.pause()
    → Broadcasts WidgetUpdateMessage (triggers re-render of entire span group)
```

---

## 5. Shared State Mechanism

### 5.1 Crate-Level Static Registry

Each plugin crate that needs span group shared state declares a module-level static:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

/// Registry of shared span group states, keyed by span_group name.
///
/// Because all instances of the same .so share the same static, this
/// allows multiple widget instances in the same span group to access
/// the same `Arc<Mutex<T>>`.
static SPAN_STATE_REGISTRY: RwLock<HashMap<String, Arc<Mutex<SpanGroupState>>>> = RwLock::new(HashMap::new());
```

### 5.2 State Lookup on Construction

When a widget instance is constructed, it checks its config for a `span_group`. If present, it looks up or creates the shared state in the registry:

```rust
impl ClockAtomicWidget {
    fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, ...> {
        // ... parse config, determine view ...

        let span_group = config.config.get("span_group").and_then(|v| v.as_str()).map(|s| s.to_string());

        let shared_state = if let Some(ref group) = span_group {
            let mut registry = SPAN_STATE_REGISTRY.write().unwrap();
            registry.entry(group.clone())
                .or_insert_with(|| Arc::new(Mutex::new(SpanGroupState::default())))
                .clone()
        } else {
            Arc::new(Mutex::new(SpanGroupState::default()))
        };

        // Store shared_state in the widget struct
        // ...
    }
}
```

### 5.3 State Cleanup on Destruction

When a widget instance is dropped, the registry entry should be cleaned up if no more instances reference it. This can be done via `Arc::strong_count`:

```rust
impl Drop for ClockAtomicWidget {
    fn drop(&mut self) {
        if let Some(ref group) = self.span_group {
            if let Ok(mut registry) = SPAN_STATE_REGISTRY.write() {
                if let Some(arc) = registry.get(group) {
                    if Arc::strong_count(arc) <= 1 {
                        registry.remove(group);
                    }
                }
            }
        }
    }
}
```

### 5.4 FFI Safety

- The static registry lives in the plugin crate's memory space, not across the FFI boundary.
- All access is through `RwLock`/`Mutex` — thread-safe.
- The `Arc<Mutex<T>>` is cloned within the same process — no pointer passing across FFI.
- The host never accesses the shared state directly — only plugin instances do.
- Since `libloading` returns the same `Library` handle for the same `.so` path, all instances share the same static.

---

## 6. Custom Action Handling

### 6.1 Current Flow (Config-Driven Only)

```
Host button press
  → InvokeToolMessage{action: "click"}
  → MessageHandler<InvokeToolMessage>::handle_message()
  → self.dispatch_action(AtomicAction::Click)
  → AtomicWidgetConfig::dispatch_action()
  → reads click_topic / click_payload
  → broadcasts message to topic
```

### 6.2 Extended Flow (Custom Action Hook)

The `atomic_widget_impl!` macro's `dispatch_action` method is extended to call an optional **`on_action`** hook on the widget before falling back to
config-driven dispatch:

```
Host button press
  → InvokeToolMessage{action: "click"}
  → MessageHandler<InvokeToolMessage>::handle_message()
  → self.dispatch_action(AtomicAction::Click)
  → AtomicWidgetConfig::dispatch_action()  (config-driven, unchanged)
      ↓
  Additionally, the widget implements a new trait:

  trait SpanActionHandler {
      fn on_span_action(&self, action: AtomicAction, span_index: u32);
  }

  The macro calls self.on_span_action(action, span_index) if the widget
  implements SpanActionHandler, before or after the config-driven dispatch.
```

### 6.3 SpanActionHandler Trait

A new optional trait that widgets can implement to react to actions with internal logic:

```rust
/// Hook for widgets that need to react to MacroPad actions with internal logic.
///
/// Implemented by widgets that manage internal state (e.g. Timer, Countdown).
/// Called by the atomic widget macro's `dispatch_action` method, giving the
/// widget an opportunity to update its state before the config-driven
/// topic/payload dispatch runs.
///
/// The `span_index` parameter identifies which button in the span group
/// was pressed, allowing per-button action differentiation.
pub trait SpanActionHandler {
    /// Called when a MacroPad action is dispatched to this widget instance.
    ///
    /// `action` is the trigger type (click, longpress, etc.).
    /// `span_index` is this instance's index within its span group (0 if not in a group).
    fn on_span_action(&self, action: AtomicAction, span_index: u32);
}
```

### 6.4 Macro Integration

The `atomic_widget_impl!` macro's generated `dispatch_action` method is extended:

```rust
pub fn dispatch_action(&self, action: AtomicAction) {
    // Call custom handler if the widget implements SpanActionHandler.
    // This is done via a blanket trait or auto-detection.
    // The widget's on_span_action can update shared state, start/stop timers, etc.
    
    // Config-driven dispatch (existing, unchanged):
    self.config.dispatch_action(&MessageBroadcaster::get_broadcaster(self), action);
}
```

The `span_index` is read from the widget's config (passed through `PluginConfig` at construction time). The widget stores it as a field.

### 6.5 Per-Button Action Mapping

With `span_index` available in `on_span_action`, the widget can differentiate actions per button. The mapping is **widget-defined** — each widget decides what
action to perform for which `span_index` and `AtomicAction` combination:

```rust
impl SpanActionHandler for ClockAtomicWidget {
    fn on_span_action(&self, action: AtomicAction, span_index: u32) {
        if self.view == ClockAtomicView::Timer {
            match (action, span_index) {
                (AtomicAction::Click, 0) => self.shared_state.lock().unwrap().timer_start(),
                (AtomicAction::Click, 1) => self.shared_state.lock().unwrap().timer_pause(),
                (AtomicAction::Click, 2) => self.shared_state.lock().unwrap().timer_reset(),
                _ => {}
            }
            self.broadcast_widget_update();
        }
    }
}
```

---

## 7. Configuration Format

### 7.1 Span Action Bindings

Per-button actions are configured using the existing `click_topic`/`longpress_topic` mechanism — **no new config fields needed**. The difference is that the
widget's `on_span_action` hook reacts **in addition** to the config-driven dispatch.

For widgets that are purely state-managing (Timer, Countdown), the config-driven dispatch can be omitted (no `click_topic`), and the widget relies entirely on
`on_span_action`:

```toml
# Timer Widget — 1×3 span
[timer_0]
render_mode = "graphic_only"
# No click_topic — on_span_action handles it

[timer_1]
render_mode = "graphic_only"
# No click_topic — on_span_action handles it

[timer_2]
render_mode = "graphic_only"
# No click_topic — on_span_action handles it
```

For widgets that want **both** internal state changes and external messaging (e.g. VolumeSpan with mute toggle broadcast):

```toml
[vol_span_0]
render_mode = "graphic_only"
click_topic = "service.audio.command"
click_payload = { action = "VolumeDown" }
# on_span_action also updates internal volume bar state

[vol_span_1]
render_mode = "graphic_only"
click_topic = "service.audio.command"
click_payload = { action = "VolumeUp" }
```

### 7.2 Span Index in Config

The `span_index` is already part of `PluginEntry` and passed to the plugin constructor via `PluginConfig`. The widget reads it at construction time:

```rust
let span_index = config.config.get("span_index")
.and_then( | v| v.as_u64())
.map( | v| v as u32)
.unwrap_or(0);
```

---

## 8. Timer Widget Implementation

### 8.1 Concept

A **stopwatch** that counts up from `00:00`. The display shows `MM:SS` (or `HH:MM:SS` after 60 minutes). The timer runs in the widget's update thread (reusing
the existing `start_time_update` mechanism).

### 8.2 State

```rust
/// State of the Timer (stopwatch) widget.
#[derive(Clone, Debug, Default)]
pub struct TimerState {
    /// Whether the timer is running, paused, or idle.
    pub status: TimerStatus,
    /// Elapsed time accumulated while running.
    pub elapsed: Duration,
    /// Instant when the current run segment started (for computing live elapsed).
    pub last_start: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimerStatus {
    #[default]
    Idle,
    Running,
    Paused,
}
```

### 8.3 Actions

| Span Index | Action             | Effect                                |
|------------|--------------------|---------------------------------------|
| 0          | Click              | Start (if Idle) or Resume (if Paused) |
| 1          | Click              | Pause (if Running)                    |
| 2          | Click              | Reset to `00:00` (always)             |
| Any        | Longpress          | No-op (reserved for future)           |
| All        | Compound Longpress | Reset to `00:00`                      |

### 8.4 Rendering

The timer display is rendered across the combined span dimensions. The `render_graphic` method reads the shared state and formats the elapsed time:

```rust
impl AtomicGraphicRenderer for ClockAtomicWidget {
    fn render_graphic(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        if self.view != ClockAtomicView::Timer {
            return false;
        }

        let state = self.shared_state.lock().unwrap();
        let elapsed = state.current_elapsed();
        let display = format_elapsed(elapsed);

        fill_background(pixels, width, height, background_color(false));
        let font_size = (height as f32 * 0.6).min(width as f32 * 0.15).min(48.0).max(20.0);
        draw_text_centered(pixels, width, height, &display, height as f32 * 0.55, font_size, text_color(false));

        // Draw status indicator (small icon or colour tint)
        match state.status {
            TimerStatus::Running => { /* green tint or play icon */ }
            TimerStatus::Paused => { /* yellow tint or pause icon */ }
            TimerStatus::Idle => { /* grey tint or stop icon */ }
        }

        true
    }
}
```

### 8.5 Update Thread

The existing `start_time_update` thread ticks every second. For the Timer view, instead of reading the wall clock, it reads the shared `TimerState` and
broadcasts a `WidgetUpdateMessage` to trigger re-render — but **only if the timer is running**:

```rust
// In start_time_update async loop:
if self .view == ClockAtomicView::Timer {
let state = self.shared_state.lock().unwrap();
if state.status == TimerStatus::Running {
broadcaster.broadcast_message_to_topic(WidgetUpdateMessage::new(...));
}
}
```

### 8.6 Example Configuration

```toml
plugins = [
    { id = "timer_0", path = "target/release/libsmearor_clock_widget.so",
      widget = "clock_timer", span_group = "timer_group", span_index = 0 },
    { id = "timer_1", path = "target/release/libsmearor_clock_widget.so",
      widget = "clock_timer", span_group = "timer_group", span_index = 1 },
    { id = "timer_2", path = "target/release/libsmearor_clock_widget.so",
      widget = "clock_timer", span_group = "timer_group", span_index = 2 },
]

[timer_0]
render_mode = "graphic_only"

[timer_1]
render_mode = "graphic_only"

[timer_2]
render_mode = "graphic_only"
```

### 8.7 Layout (1×3)

```
┌────────┬────────┬────────┐
│        │        │        │
│   ▶    │  00:23 │  ⏹     │
│ Start  │ Timer  │ Reset  │
│        │        │        │
└────────┴────────┴────────┘
 Button 0  Button 1  Button 2

Click (0): Start / Resume
Click (1): Pause
Click (2): Reset
```

The combined graphic shows the elapsed time centered across all three buttons, with status indicators on the left and right buttons.

---

## 9. Countdown Widget Implementation

### 9.1 Concept

A **countdown timer** that counts down from a configured time toward `00:00`. When it reaches zero, it stops and displays `00:00` (optionally with a visual
alert). The user can increment the target time before starting.

### 9.2 State

```rust
/// State of the Countdown widget.
#[derive(Clone, Debug)]
pub struct CountdownState {
    /// Whether the countdown is running, paused, or idle.
    pub status: CountdownStatus,
    /// Target duration to count down from.
    pub target: Duration,
    /// Remaining time.
    pub remaining: Duration,
    /// Instant when the current run segment started.
    pub last_start: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CountdownStatus {
    #[default]
    Idle,
    Running,
    Paused,
    Finished,
}
```

### 9.3 Actions

| Span Index | Action             | Effect                                                        |
|------------|--------------------|---------------------------------------------------------------|
| 0          | Click              | Increase target by 1 minute (if Idle or Finished)             |
| 1          | Click              | Increase target by 1 second (if Idle or Finished)             |
| 0          | Longpress          | Start countdown (if Idle or Finished)                         |
| 1          | Longpress          | Toggle pause (if Running or Paused)                           |
| 2          | Click              | Reset to target (if Running/Paused) or clear target (if Idle) |
| All        | Compound Longpress | Reset to `00:00` and clear target                             |

### 9.4 Rendering

```rust
impl AtomicGraphicRenderer for ClockAtomicWidget {
    fn render_graphic(&self, pixels: &mut [u8], width: u32, height: u32) -> bool {
        if self.view != ClockAtomicView::Countdown {
            return false;
        }

        let state = self.shared_state.lock().unwrap();
        let remaining = state.current_remaining();
        let display = format_duration(remaining);

        fill_background(pixels, width, height, background_color(false));
        let font_size = (height as f32 * 0.6).min(width as f32 * 0.15).min(48.0).max(20.0);
        draw_text_centered(pixels, width, height, &display, height as f32 * 0.55, font_size, text_color(false));

        // Status indicator
        match state.status {
            CountdownStatus::Running => { /* green tint or play icon */ }
            CountdownStatus::Paused => { /* yellow tint or pause icon */ }
            CountdownStatus::Idle => { /* grey tint or clock icon */ }
            CountdownStatus::Finished => { /* red flash or bell icon */ }
        }

        true
    }
}
```

### 9.5 Update Thread

Same as Timer — the existing per-second tick thread checks if the countdown is running and broadcasts a `WidgetUpdateMessage`. When `remaining` hits zero, the
state transitions to `Finished`:

```rust
if self .view == ClockAtomicView::Countdown {
let mut state = self.shared_state.lock().unwrap();
if state.status == CountdownStatus::Running {
let elapsed_since_start = state.last_start.map( |s | s.elapsed()).unwrap_or_default();
state.remaining = state.target.saturating_sub(elapsed_since_start);
if state.remaining == Duration::ZERO {
state.status = CountdownStatus::Finished;
}
}
drop(state);
broadcaster.broadcast_message_to_topic(WidgetUpdateMessage::new(...));
}
```

### 9.6 Example Configuration

```toml
plugins = [
    { id = "countdown_0", path = "target/release/libsmearor_clock_widget.so",
      widget = "clock_countdown", span_group = "countdown_group", span_index = 0 },
    { id = "countdown_1", path = "target/release/libsmearor_clock_widget.so",
      widget = "clock_countdown", span_group = "countdown_group", span_index = 1 },
    { id = "countdown_2", path = "target/release/libsmearor_clock_widget.so",
      widget = "clock_countdown", span_group = "countdown_group", span_index = 2 },
]

[countdown_0]
render_mode = "graphic_only"

[countdown_1]
render_mode = "graphic_only"

[countdown_2]
render_mode = "graphic_only"
```

### 9.7 Layout (1×3)

```
┌────────┬────────┬────────┐
│        │        │        │
│  +1m   │ 05:00  │  Reset │
│  ⏯     │ Count  │  ⏹     │
│        │        │        │
└────────┴────────┴────────┘
 Button 0  Button 1  Button 2

Click (0):    +1 minute (if idle)
Click (1):    +1 second (if idle)
Click (2):    Reset
Longpress (0): Start
Longpress (1): Toggle pause
Compound Longpress: Reset to 00:00
```

---

## 10. Re-rendering on State Change

### 10.1 Problem

When any instance in a span group changes the shared state, the **entire span group** must be re-rendered. The host's `render_single_button_to_device` already
handles this — when a plugin broadcasts `WidgetUpdateMessage`, the host finds the plugin, detects its `span_group`, and re-renders all members.

### 10.2 Solution

When `on_span_action` modifies the shared state, the instance calls `self.broadcast_widget_update()`. The host receives the `WidgetUpdateMessage`, looks up the
plugin's `span_group`, and re-renders the entire group at combined dimensions.

However, there is a subtlety: **only the first member** (lowest `span_index`) renders the graphic. The host calls `render_graphic` on the first member only.
This means the first member's `render_graphic` must read the shared state and produce the complete combined image.

Since all instances share the same `Arc<Mutex<State>>`, the first member can read the state regardless of which instance triggered the update. The
`WidgetUpdateMessage` from any member triggers a full group re-render.

### 10.3 Update Thread Consideration

The per-second tick thread runs in **each instance**. For span group widgets, only the first member's tick thread should drive updates to avoid redundant
re-render broadcasts. Alternatively, all instances broadcast, and the host deduplicates (it already re-renders the entire group on any member's update, so
multiple broadcasts in the same frame are harmless but wasteful).

**Recommendation**: Only the first member (span_index == 0) runs the tick-driven broadcast for Timer/Countdown views. Other instances are passive.

---

## 11. Generalisation for Other Widgets

### 11.1 VolumeSpan Example

The same mechanism applies to a 1×3 VolumeSpan:

```rust
impl SpanActionHandler for AudioAtomicWidget {
    fn on_span_action(&self, action: AtomicAction, span_index: u32) {
        if self.view == AudioAtomicView::VolumeSpan {
            match (action, span_index) {
                (AtomicAction::Click, 0) => {
                    // Volume down — also broadcasts via config-driven dispatch
                }
                (AtomicAction::Click, 1) => {
                    // Toggle mute — config-driven dispatch handles the topic
                }
                (AtomicAction::Click, 2) => {
                    // Volume up — also broadcasts via config-driven dispatch
                }
                _ => {}
            }
            self.broadcast_widget_update();
        }
    }
}
```

### 11.2 Pattern Summary

| Step               | What Happens                                                                                                                 |
|--------------------|------------------------------------------------------------------------------------------------------------------------------|
| 1. Construction    | Widget reads `span_group` and `span_index` from config. Looks up or creates shared state in the crate-level static registry. |
| 2. Button press    | Host sends `InvokeToolMessage{action}` to the specific instance at that button index.                                        |
| 3. Action handling | Instance's `on_span_action(action, span_index)` updates the shared state based on its `span_index`.                          |
| 4. Config dispatch | `AtomicWidgetConfig::dispatch_action()` forwards to the configured topic (if any).                                           |
| 5. Re-render       | Instance calls `broadcast_widget_update()`. Host re-renders the entire span group.                                           |
| 6. Render          | First member's `render_graphic()` reads shared state and produces the combined image.                                        |

---

## 12. Implementation Phases

### Phase 1: SpanActionHandler Trait & Macro Integration

**Order**: First — provides the foundation for all interactive span group widgets.

**Changes**:

- Add `SpanActionHandler` trait to `plugin-api/src/atomic/action.rs`.
- Add `span_index: u32` field to widget structs (read from config at construction).
- Extend `atomic_widget_impl!` macro to call `on_span_action` before config-driven dispatch.
- Add `SpanGroupState` type to `plugin-api/src/atomic/` (generic enum or trait object for shared state).

**Exit Criteria**: Widgets implementing `SpanActionHandler` receive `on_span_action` calls on button presses with correct `span_index`.

### Phase 2: Shared State Registry

**Order**: After Phase 1.

**Changes**:

- Add `SPAN_STATE_REGISTRY: RwLock<HashMap<String, Arc<Mutex<SpanGroupState>>>>` to the clock plugin crate.
- Implement state lookup/creation in `ClockAtomicWidget::new()`.
- Implement state cleanup in `Drop` for `ClockAtomicWidget`.
- Add `span_group: Option<String>` and `span_index: u32` fields to `ClockAtomicWidget`.

**Exit Criteria**: Multiple instances with the same `span_group` share the same `Arc<Mutex<SpanGroupState>>`. Verified by unit test or debug logging.

### Phase 3: Timer Widget

**Order**: After Phase 2.

**Changes**:

- Add `TimerState` and `TimerStatus` to `plugins/clock/src/timer_state.rs`.
- Implement `TimerState` methods: `start()`, `pause()`, `reset()`, `current_elapsed()`.
- Implement `SpanActionHandler` for `ClockAtomicWidget` (Timer view only).
- Update `render_graphic` to display elapsed time from shared state.
- Update `start_time_update` to broadcast updates only when timer is running and only from span_index == 0.
- Add `format_elapsed` helper function.

**Exit Criteria**: Timer widget starts, pauses, and resets correctly via button presses. Display updates every second while running. Re-render covers the entire
span group.

### Phase 4: Countdown Widget

**Order**: After Phase 3 (reuses the same infrastructure).

**Changes**:

- Add `CountdownState` and `CountdownStatus` to `plugins/clock/src/countdown_state.rs`.
- Implement `CountdownState` methods: `increment_minutes()`, `increment_seconds()`, `start()`, `toggle_pause()`, `reset()`, `current_remaining()`.
- Implement `SpanActionHandler` for `ClockAtomicWidget` (Countdown view only).
- Update `render_graphic` to display remaining time from shared state.
- Update `start_time_update` to tick the countdown and detect completion.
- Add `format_duration` helper function.

**Exit Criteria**: Countdown widget increments, starts, pauses, and resets correctly. Countdown reaches zero and transitions to `Finished` state. Display
updates every second while running.

### Phase 5: Configuration & Testing

**Order**: After Phase 4.

**Changes**:

- Update `streamcontrollerx.toml` and `streamdeck.toml` clock_area configs to use 1×3 spans for Timer and Countdown.
- Add unit tests for `TimerState` and `CountdownState` state transitions.
- Add unit tests for `SpanActionHandler` action mapping.
- Test with physical devices: verify per-button actions, shared state consistency, and re-rendering.

**Exit Criteria**: Timer and Countdown widgets work correctly on Stream Deck and Stream Controller X with 1×3 span configuration. All state transitions are
correct. No stale state after area switching.

---

## 13. Edge Cases

### 13.1 Area Switching

When the user navigates away from the clock_area and back, plugin instances are **not** destroyed — they persist in the `PluginManager`. The shared state
remains valid. The timer/countdown continues running in the background.

If the user navigates away and the update thread keeps broadcasting `WidgetUpdateMessage`, the host's `render_single_button_to_device` will find the plugin is
not in the visible area and skip rendering. This is harmless.

### 13.2 Multiple Span Groups

If two separate timer span groups exist (e.g. `timer_group_1` and `timer_group_2`), each gets its own entry in the `SPAN_STATE_REGISTRY`. They are completely
independent.

### 13.3 Instance Without Span Group

A widget instance without a `span_group` (e.g. a standalone atomic clock widget) gets its own private `Arc<Mutex<SpanGroupState>>`. It is not shared with any
other instance. The `SpanActionHandler` still works — `span_index` defaults to 0.

### 13.4 Library Reload

If the host unloads and reloads a plugin `.so` (e.g. on config change), the `Library` handle is dropped and recreated. The static registry is reinitialised to
an empty `HashMap`. All shared state is lost. This is acceptable — reloading implies a fresh start.

### 13.5 Race Condition: Tick vs. Action

The per-second tick thread and the `on_span_action` handler both access the shared state via `Mutex`. The lock duration is minimal (read state, compute elapsed,
release). No deadlock risk since no nested locks are held.

### 13.6 First Member Not Loaded

If the first member (span_index == 0) fails to load but other members succeed, the host's span group rendering will fail (it renders via the first member). The
shared state registry will have an entry (created by whichever member loaded first), but no rendering occurs. This is a configuration error — the host should
log a warning.

---

## 14. Security Considerations

- The static registry is process-local and not exposed via FFI.
- No user-supplied input is processed for span group state — all actions come from physical button presses routed through the host.
- The `Mutex` ensures thread-safe access — no data races.

---

## 15. Performance Considerations

- `Mutex` lock duration is minimal (microseconds for state read/update).
- The per-second tick thread runs per instance but only span_index == 0 broadcasts for Timer/Countdown, avoiding redundant re-renders.
- `Arc::strong_count` check in `Drop` is O (1).
- The static registry `HashMap` lookup is O (1) by span_group string key.
- No additional allocations per frame — the shared state is read, not cloned, during rendering.

---

## 16. Dependencies

| Dependency             | Type           | Required For                                    |
|------------------------|----------------|-------------------------------------------------|
| `plugin-api`           | Existing crate | `SpanActionHandler` trait, macro extension      |
| `plugins/clock`        | Existing crate | Timer/Countdown implementation, static registry |
| `model/plugin`         | Existing crate | `span_group`/`span_index` (already present)     |
| `plugins/render-utils` | Existing crate | Drawing functions (already used)                |

No new crate dependencies are introduced.
