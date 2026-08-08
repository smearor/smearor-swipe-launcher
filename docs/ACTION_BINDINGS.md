# Action Binding Status — All Widgets

Legend: ✅ = supported & dispatch-respecting `instance` | ⚠️ = hardcoded default only, no config binding | ❌ = not supported | — = N/A

> **Note:** `button`, `audio`, `mpris`, `weather`, `clock`, `power`, `wallpaper`, `network`, `app-launcher`, `voice_assistant`, `workspace-switcher`,
> `notifications`, and `sysinfo` (all 7 sub-widgets) use the shared `ActionBindings` struct from `plugin-api` (embedded via `#[serde(flatten)]`). All other
> widgets still declare individual `ClickBinding` / `LongpressBinding` fields.

> **Gesture Handler Architecture:** `app-launcher`, `audio`, `power`, `mpris`, and `button` use the centralized `attach_gesture_handlers` trait method from
> `plugin-api::gesture`. This provides a unified `GestureHandlersConfiguration` with options for `swipe_threshold`, `delay_factor`, `group_gestures`,
> `longpress_css_class`, `drag_throttling`, `drag_enabled`, and `scroll_throttling`. Widgets implement `DefaultFallback` (with optional
> `default_fallback_with_button` and `default_fallback_drag` overrides) to define fallback behavior. Widgets with custom drag logic (e.g. `button` with template
> resolution) set `drag_enabled: false` and attach their own `GestureDrag` inline.

> **BindingMode:** All binding structs (`ClickBinding`, `LongpressBinding`, etc.) include a `mode` field of type `BindingMode` (`replace` or `supplement`). In
> `replace` mode (default), a configured binding replaces the widget's default fallback behavior. In `supplement` mode, both the binding **and** the default
> fallback are dispatched. This allows e.g. configuring a `click` binding that sends a message while still launching the app, or a `longpress` binding that also
> closes the menu. Per-binding TOML config: `click_mode = "supplement"`. Supported by: `app-launcher`, `network`, `wallpaper`, `weather`, `mpris`, `audio`,
> `power`, `voice_assistant`, `workspace-switcher`, `notifications`, `sysinfo`, `button`. `button` has a no-op fallback (no default actions), so `supplement`
> mode
> effectively only dispatches the binding.

## Config Bindings

Which `*Binding` structs exist in each widget's config.

| Widget                      | Click | Longpress | Hold | DoublePress | SwipeUp | SwipeDown | RightClick | MiddleClick | ScrollUp | ScrollDown | CompoundLongpress | Init |
|-----------------------------|-------|-----------|------|-------------|---------|-----------|------------|-------------|----------|------------|-------------------|------|
| **button**                  | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | ✅   |
| **atomic**                  | ✅    | ✅        | ✅   | ✅          | —       | —         | —          | —           | —        | —          | ✅                | —    |
| **audio**                   | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **mpris**                   | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **weather**                 | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **clock**                   | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **workspace-switcher**      | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **power**                   | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **wallpaper**               | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **network**                 | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **app-launcher**            | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **voice_assistant**         | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **notifications**           | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |
| **sysinfo** (7 sub-widgets) | ✅    | ✅        | ✅   | ✅          | ✅      | ✅        | ✅         | ✅          | ✅       | ✅         | ✅                | —    |

## `handle_message` (MCP/LLM Tool Invocation)

How each widget handles `InvokeToolMessage` — whether it parses `ActionKind`, uses `dispatch()`, respects `instance`, and has a default fallback.

| Widget                 | ActionKind parsing          | dispatch()            | instance respected | Default fallback                                                                                                                                                                                                                               |
|------------------------|-----------------------------|-----------------------|--------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **button**             | ✅ `ActionKind::from_str`   | ✅ `dispatch_by_kind` | ✅                 | no-op (no default actions)                                                                                                                                                                                                                     |
| **atomic**             | ✅ `AtomicAction::from_str` | ✅ `dispatch_action`  | ✅                 | —                                                                                                                                                                                                                                              |
| **audio**              | ✅ `ActionKind::from_str`   | ✅ `binding_for_kind` | ✅                 | `toggle_mute` (double_press) / `next_device` (longpress) / `previous_device` (right longpress) / `unmute` (middle longpress) / `volume_up` / `volume_down` (swipe+scroll) / proportional drag volume                                           |
| **mpris**              | ✅ `ActionKind::from_str`   | ✅ `binding_for_kind` | ✅                 | `toggle_play_pause` (double_press) / `next_player` (longpress) / `previous_player` (right longpress) / `toggle_play_pause` (middle longpress) / `next_track` / `previous_track` (swipe+scroll) / `raise` (right-click) / `quit` (middle-click) |
| **weather**            | ✅ `ActionKind::from_str`   | ✅ `binding_for_kind` | ✅                 | `next_view` / `refresh` / `prev_view` (click/longpress/double_press/swipe+scroll/right/middle)                                                                                                                                                 |
| **clock**              | ✅ `ActionKind::from_str`   | ✅ `binding_for_kind` | ✅                 | `get_current_time` tool / `toggle_format` (click/double_press/swipe+scroll/right/middle)                                                                                                                                                       |
| **workspace-switcher** | —                           | —                     | —                  | —                                                                                                                                                                                                                                              |
| **power**              | —                           | —                     | —                  | `execute` (click/double-click) / `next_view` (swipe_up/scroll_up/middle-click) / `prev_view` (swipe_down/scroll_down)                                                                                                                          |
| **wallpaper**          | ✅ `ActionKind::from_str`   | ✅ `binding_for_kind` | ✅                 | `start_selected` / `stop_current` / `select_prev_theme` / `select_next_theme` (click/longpress/double_press/swipe+scroll/right/middle)                                                                                                         |
| **network**            | ✅ `ActionKind::from_str`   | ✅ `binding_for_kind` | ✅                 | `handle_click` / `refresh` / `next_view` / `prev_view` (click/longpress/double_press/swipe+scroll/right/middle)                                                                                                                                |
| **app-launcher**       | ✅ `ActionKind::from_str`   | ✅ `binding_for_kind` | ✅                 | `exec` / `terminate` (click/longpress/double_press/right_click)                                                                                                                                                                                |
| **voice_assistant**    | —                           | —                     | —                  | —                                                                                                                                                                                                                                              |
| **notifications**      | —                           | —                     | —                  | —                                                                                                                                                                                                                                              |
| **sysinfo**            | —                           | —                     | —                  | —                                                                                                                                                                                                                                              |

## GTK Gesture Handlers — Click

| Widget                 | Gesture        | dispatch() | instance | Default fallback                                                                                         |
|------------------------|----------------|------------|----------|----------------------------------------------------------------------------------------------------------|
| **button**             | `GestureClick` | ✅         | ✅       | no-op (no default actions)                                                                               |
| **atomic**             | (via macro)    | ✅         | ✅       | —                                                                                                        |
| **audio**              | `GestureClick` | ✅         | ✅       | `toggle_mute` (double-click), `next_device` (secondary) — single-click is no-op                          |
| **mpris**              | `GestureClick` | ✅         | ✅       | `toggle_play_pause` (double-click), `raise` (right-click), `quit` (middle-click) — single-click is no-op |
| **weather**            | `GestureClick` | ✅         | ✅       | `next_view` (click/double-click), `refresh` (right-click), `prev_view` (middle-click)                    |
| **clock**              | `GestureClick` | ✅         | ✅       | `toggle_format` (click/double-click), `toggle_format` (right-click), `toggle_format` (middle-click)      |
| **workspace-switcher** | `GestureClick` | ✅         | ✅       | `next_view` (middle-click)                                                                               |
| **power**              | `GestureClick` | ✅         | ✅       | `execute` (click/double-click), `next_view` (middle-click)                                               |
| **wallpaper**          | `GestureClick` | ✅         | ✅       | `start_selected` (click/double-click), `stop_current` (right-click)                                      |
| **network**            | `GestureClick` | ✅         | ✅       | `handle_click` (click/double-click), `refresh` (right-click), `next_view` (middle-click)                 |
| **app-launcher**       | `GestureClick` | ✅         | ✅       | `exec` (click/double-click), `terminate` (right-click)                                                   |
| **voice_assistant**    | `GestureClick` | ✅         | ✅       | `activate` (click/double-click), `deactivate` (right-click)                                              |
| **notifications**      | `GestureClick` | ✅         | ✅       | `dismiss_all` (click/double-click/right-click), `dismiss_last` (middle-click)                            |
| **sysinfo**            | `GestureClick` | ✅         | ✅       | — (display-only, no fallback)                                                                            |

## GTK Gesture Handlers — Longpress

| Widget                 | Gesture            | dispatch() | instance | Default fallback                                                                                             |
|------------------------|--------------------|------------|----------|--------------------------------------------------------------------------------------------------------------|
| **button**             | `GestureLongPress` | ✅         | ✅       | no-op (no default actions)                                                                                   |
| **atomic**             | (via macro)        | ✅         | ✅       | —                                                                                                            |
| **audio**              | `GestureLongPress` | ✅         | ✅       | `next_device` / `previous_device` / `unmute` (button-specific via `default_fallback_with_button`)            |
| **mpris**              | `GestureLongPress` | ✅         | ✅       | `next_player` / `previous_player` / `toggle_play_pause` (button-specific via `default_fallback_with_button`) |
| **weather**            | `GestureLongPress` | ✅         | ✅       | `refresh`                                                                                                    |
| **clock**              | `GestureLongPress` | ✅         | ✅       | `get_current_time` tool (no default, configurable only)                                                      |
| **workspace-switcher** | `GestureLongPress` | ✅         | ✅       | —                                                                                                            |
| **power**              | `GestureLongPress` | ✅         | ✅       | no-op (only if configured)                                                                                   |
| **wallpaper**          | `GestureLongPress` | ✅         | ✅       | `stop_current`                                                                                               |
| **network**            | `GestureLongPress` | ✅         | ✅       | `refresh`                                                                                                    |
| **app-launcher**       | `GestureLongPress` | ✅         | ✅       | `terminate`                                                                                                  |
| **voice_assistant**    | `GestureLongPress` | ✅         | ✅       | `deactivate`                                                                                                 |
| **notifications**      | `GestureLongPress` | ✅         | ✅       | `dismiss_last` (widget-level); `dismiss_id` (per-card, hardcoded)                                            |
| **sysinfo**            | `GestureLongPress` | ✅         | ✅       | — (display-only, no fallback)                                                                                |

## GTK Gesture Handlers — Swipe (Up/Down)

| Widget                 | SwipeUp | SwipeDown | dispatch() | instance | Notes                                                                                                                                                               |
|------------------------|---------|-----------|------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **button**             | ✅      | ✅        | ⚠️ manual  | ✅       | Custom `GestureDrag` with `resolve_payload_template` against state (standard drag disabled via `drag_enabled: false`); scroll via standard handler (no-op fallback) |
| **atomic**             | —       | —         | —          | —        | Not supported                                                                                                                                                       |
| **audio**              | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = proportional volume change (`default_fallback_drag`); scroll = volume up/down (throttled 150ms)                                                              |
| **mpris**              | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = next/previous_track (throttled 150ms); scroll = next/previous_track (throttled 150ms); GestureZoom = raise/quit (inline)                                     |
| **weather**            | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = next/prev_view (fallback); scroll uses separate `scroll_up`/`scroll_down` bindings                                                                           |
| **clock**              | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = toggle_format (fallback); scroll uses separate `scroll_up`/`scroll_down` bindings                                                                            |
| **workspace-switcher** | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = next/prev_view (fallback); scroll uses separate `scroll_up`/`scroll_down` bindings                                                                           |
| **power**              | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = next/prev_view (fallback); scroll uses separate `scroll_up`/`scroll_down` bindings                                                                           |
| **wallpaper**          | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = select_prev/next_theme (fallback); scroll uses separate `scroll_up`/`scroll_down` bindings                                                                   |
| **network**            | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = next/prev_view (fallback); scroll uses separate `scroll_up`/`scroll_down` bindings                                                                           |
| **app-launcher**       | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = no fallback (configurable only); scroll uses separate `scroll_up`/`scroll_down` bindings (no fallback)                                                       |
| **voice_assistant**    | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = no fallback (configurable only); scroll uses separate `scroll_up`/`scroll_down` bindings (no fallback)                                                       |
| **notifications**      | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = toggle_do_not_disturb (swipe_up fallback); scroll uses separate `scroll_up`/`scroll_down` bindings                                                           |
| **sysinfo**            | ✅ drag | ✅ drag   | ✅         | ✅       | Drag = no fallback (display-only); scroll uses separate `scroll_up`/`scroll_down` bindings (no fallback)                                                            |

## Remaining Gaps

- **Gesture handler architecture**: All widgets (`app-launcher`, `audio`, `power`, `mpris`, `button`, `weather`, `clock`, `wallpaper`, `network`,
  `voice_assistant`, `workspace-switcher`, `notifications`, `sysinfo`) now use the centralized `attach_gesture_handlers` trait method with
  `GestureHandlersConfiguration`.
- **Swipe gestures**: All widgets with `ActionBindings` have configurable swipe bindings. `button` uses a custom drag handler with template resolution (standard
  drag disabled).
- **`button_` prefix convention**: All `InvokeToolMessage` handlers use `format!("button_{}", self.meta.id)` as tool name, copied from the `button` widget. Only
  the `button` widget actually registers this tool via `RegisterToolMessage`. Other widgets register their own tools (e.g. `get_current_time`,
  `weather_widget_refresh`). The `button_` prefix is practically relevant only for MacroPad input dispatch (`host/mod.rs`). Consider renaming to
  `widget_{plugin_id}` for clarity.
