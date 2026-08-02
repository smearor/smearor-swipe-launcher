# Swipe and Touch Navigation

The launcher is designed for touch-first interaction. Swipe gestures, long-press, and double-press are native interaction patterns available on all widgets.

## Swipe Gestures

| Gesture     | Action                      |
|-------------|-----------------------------|
| Swipe Left  | Scroll left / Previous page |
| Swipe Right | Scroll right / Next page    |
| Swipe Up    | Open parent menu            |
| Swipe Down  | Scroll / Close              |

Swipe gestures work with both touch and mouse input. Keyboard navigation (`SUPER + ARROW KEYS`) is also supported.

## Gesture Handler Architecture

Widgets use the centralized `attach_gesture_handlers` trait method from `plugin-api::gesture`. This provides a unified `GestureHandlersConfiguration` with
options for:

- `swipe_threshold` — Minimum distance to register a swipe
- `delay_factor` — Animation delay scaling
- `group_gestures` — Whether to group multi-finger gestures
- `longpress_css_class` — CSS class applied during long-press
- `drag_throttling` — Throttle drag events for performance
- `drag_enabled` — Whether drag gestures are enabled
- `scroll_throttling` — Throttle scroll events

## Press Patterns

| Pattern                 | Description                                                          |
|-------------------------|----------------------------------------------------------------------|
| **Click**               | Short press and release                                              |
| **Long-press**          | Press held for ≥ 500ms                                               |
| **Double-press**        | Two clicks within a configurable window                              |
| **Hold**                | Press that triggers `hold_start` immediately, `hold_stop` on release |
| **Compound long-press** | 2+ buttons held simultaneously in a span group for ≥ 500ms           |

## Action Bindings

Each press pattern can be bound to a configurable action via [Action Bindings](./action-bindings.md). Bindings specify a topic, payload, and optional target
instance.

See [Action Bindings](./action-bindings.md) for the full binding system, and [Using Action Bindings](../plugin-api/action-bindings.md) for the API perspective.
