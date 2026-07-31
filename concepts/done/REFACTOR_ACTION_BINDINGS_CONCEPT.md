# Concept: Refactor Action Bindings

This document defines the concept for **ActionBinding** — a reusable struct that generalises the repetitive `topic` / `payload` / `instance` / `description`
field triplets found across all widget config structs. The goal is to eliminate code duplication, enforce consistency, and provide reusable dispatch logic on
the binding itself.

---

## 1. Problem Statement

### 1.1 Current State

Every widget config struct repeats the same pattern for each input trigger:

```rust
/// Message topic for single-click action.
pub click_topic: Option<String>,
/// Message payload for single-click action (JSON/TOML).
pub click_payload: Option<Value>,
/// Target instance for single-click message.
pub click_instance: Option<String>,
/// Human-readable description of what the click action does.
pub click_description: Option<String>,
```

This pattern appears for **eight action kinds** across **eleven config structs**:

| Action Kind       | Field Prefix          | Widgets Using It                 |
|-------------------|-----------------------|----------------------------------|
| Click             | `click_`              | All 11 config structs            |
| Longpress         | `longpress_`          | All 11 config structs            |
| Hold              | `hold_`               | AtomicWidgetConfig, ButtonConfig |
| DoublePress       | `double_press_`       | AtomicWidgetConfig, ButtonConfig |
| SwipeUp           | `swipe_up_`           | ButtonConfig                     |
| SwipeDown         | `swipe_down_`         | ButtonConfig                     |
| CompoundLongpress | `compound_longpress_` | AtomicWidgetConfig, ButtonConfig |
| Init              | `init_`               | ButtonConfig                     |

### 1.2 Inconsistencies

- **Missing `*_instance` fields**: `ClockConfig`, `MprisWidgetConfig`, `WeatherWidgetConfig`, `WorkspaceSwitcherConfig` lack `*_instance` fields. This means
  these widgets cannot target specific service instances — a silent limitation.
- **Missing `*_description` fields**: `AtomicWidgetConfig`, `MprisWidgetConfig`, `AudioWidgetConfig`, `WeatherWidgetConfig`, `NetworkWidgetConfig`,
  `WallpaperWidgetConfig` lack `*_description` fields. The `description` field is important for the Voice Assistant: it provides human-readable context for each
  action so the LLM can select the correct tool call. Without per-action descriptions, the LLM has no way to distinguish between, e.g. "click turns the light on
  (red)" and "longpress turns the light off".
- **Missing action kinds**: `AtomicWidgetConfig` lacks `SwipeUp` and `SwipeDown` bindings (not applicable on MacroPad, but relevant for GTK instances).
- **Repetitive dispatch logic**: `resolve_action()` and `dispatch_action()` in `AtomicWidgetConfig` and the inline match in `ButtonWidget::handle_message`
  duplicate the same topic/payload/instance extraction and broadcast pattern.

### 1.3 What Is Missing

- A **single struct** (`ActionBinding`) that encapsulates `topic`, `payload`, `instance`, and `description`.
- A **declarative macro** that generates per-action-kind wrapper structs with `#[serde(flatten)]` compatibility, using an **enum** (`ActionKind`) for the action
  kind instead of string literals.
- **Reusable logic** on `ActionBinding`: `is_configured()`, `resolve()`, `dispatch()`, and `as_tool_description()`.
- **Consistency**: All config structs gain `instance` and `description` fields for every action kind, even where they were previously missing.

---

## 2. Goals

- **Eliminate field duplication**: Replace 3–4 repetitive fields per action kind with a single flattened `ActionBinding` field.
- **Maintain TOML/JSON backward compatibility**: Existing config files (`click_topic = "..."`, `click_payload = {...}`) continue to work without any migration.
- **Enforce consistency**: All config structs gain `instance` and `description` for every action kind.
- **Provide reusable dispatch logic**: `ActionBinding` carries its own `is_configured()`, `resolve()`, and `dispatch()` methods, eliminating duplicated
  broadcast code.
- **Support the Voice Assistant**: Per-action `description` fields are available on all action kinds so the LLM can make informed tool calls.
- **Use an enum for action kinds**: The macro accepts `ActionKind` variants instead of string literals, providing compile-time safety and IDE autocomplete.

## 3. Non-Goals

- Changing the `AtomicAction` enum (used for runtime action dispatch in `plugin-api/src/atomic/action.rs`).
- Changing the `FfiEnvelope` or message passing system.
- Changing the `widget_plugin!` or `service_plugin!` macros.
- Removing the top-level `description` field on config structs (used for MCP tool registration, not per-action descriptions).
- Changing the TOML config file format.

---

## 4. ActionKind Enum

A new enum that enumerates all possible action kinds for widget input triggers. This enum is used by the declarative macro to generate per-kind wrapper structs.

```rust
/// All possible input trigger kinds for widget actions.
///
/// Used by the `action_binding!` macro to generate per-kind wrapper structs
/// with correct `#[serde(rename)]` attributes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ActionKind {
    /// Single-click action (press duration < 500 ms, fires on release).
    Click,
    /// Long-press action (press duration >= 500 ms, fires on release).
    Longpress,
    /// Hold action (push-to-talk: fires on press, stops on release).
    Hold,
    /// Double press action (two clicks within 300 ms).
    DoublePress,
    /// Swipe-up gesture (GTK touch instances only).
    SwipeUp,
    /// Swipe-down gesture (GTK touch instances only).
    SwipeDown,
    /// Compound longpress (two+ buttons in same span group held >= 500 ms).
    CompoundLongpress,
    /// Initial one-shot request (sent on widget construction).
    Init,
}

impl ActionKind {
    /// Returns the TOML field prefix for this action kind.
    ///
    /// Examples: `Click` → `"click"`, `SwipeUp` → `"swipe_up"`,
    /// `CompoundLongpress` → `"compound_longpress"`.
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Longpress => "longpress",
            Self::Hold => "hold",
            Self::DoublePress => "double_press",
            Self::SwipeUp => "swipe_up",
            Self::SwipeDown => "swipe_down",
            Self::CompoundLongpress => "compound_longpress",
            Self::Init => "init",
        }
    }
}
```

---

## 5. ActionBinding Struct

### 5.1 Definition

```rust
/// A configurable action binding for a widget input trigger.
///
/// Encapsulates the message topic, payload, target instance, and
/// human-readable description for a single action kind. Used in widget
/// config structs via `#[serde(flatten)]` with per-kind wrapper structs
/// generated by the `action_binding!` macro.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ActionBinding {
    /// Message topic to broadcast when this action is triggered.
    pub topic: Option<String>,
    /// Message payload to send with the broadcast (JSON/TOML).
    pub payload: Option<Value>,
    /// Target instance for the message. If `None`, broadcasts to all instances.
    pub instance: Option<String>,
    /// Human-readable description of what this action does.
    ///
    /// Used by the Voice Assistant LLM to select the correct tool call.
    /// Example: `"Turns the living room light on (red)"`.
    pub description: Option<String>,
}
```

### 5.2 Reusable Logic

```rust
impl ActionBinding {
    /// Returns `true` if this binding has both a topic and a payload configured.
    pub fn is_configured(&self) -> bool {
        self.topic.is_some() && self.payload.is_some()
    }

    /// Resolves this binding into a `ResolvedAction` for broadcasting.
    ///
    /// Returns `None` if the binding is not configured (missing topic or payload).
    pub fn resolve(&self) -> Option<ResolvedAction<'_>> {
        let topic = self.topic.as_ref().map(|s| s.as_str())?;
        let payload = self.payload.as_ref()?;
        let instance = self.instance.as_ref();
        Some(ResolvedAction { topic, payload, instance })
    }

    /// Dispatches this binding's action via the given broadcaster.
    ///
    /// If the binding is not configured, this is a no-op.
    /// If `instance` is set, broadcasts to that specific instance.
    /// Otherwise, broadcasts to all instances.
    pub fn dispatch(&self, broadcaster: &MessageBroadcasterInner) {
        if let Some(resolved) = self.resolve() {
            if let Some(instance) = resolved.instance {
                broadcaster.broadcast_string_to_instance(instance, resolved.topic, &resolved.payload.to_string());
            } else {
                broadcaster.broadcast_string(resolved.topic, &resolved.payload.to_string());
            }
        }
    }

    /// Returns the description if set, or `None` if not configured.
    ///
    /// Used for MCP tool registration so the Voice Assistant LLM
    /// can understand what each action does.
    pub fn as_tool_description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
```

### 5.3 ResolvedAction

The existing `ResolvedAction` struct in `plugin-api/src/atomic/action.rs` is reused unchanged:

```rust
/// The resolved routing information for a single action binding.
#[derive(Debug, Clone)]
pub struct ResolvedAction<'a> {
    /// The message topic to broadcast to.
    pub topic: &'a str,
    /// The payload to send with the broadcast.
    pub payload: &'a Value,
    /// Optional target instance for the broadcast.
    pub instance: Option<&'a String>,
}
```

---

## 6. Declarative Macro

### 6.1 `action_binding!` Macro

The macro generates a per-action-kind wrapper struct with `#[serde(rename)]` attributes that map the generic field names (`topic`, `payload`, `instance`,
`description`) to the action-specific TOML keys (e.g. `click_topic`, `click_payload`).

```rust
/// Generates a per-action-kind wrapper struct around `ActionBinding`
/// with `#[serde(rename)]` attributes for TOML/JSON field compatibility.
///
/// The generated struct is flattened into the parent config struct via
/// `#[serde(flatten)]`, preserving the existing TOML field names
/// (e.g. `click_topic`, `click_payload`, `click_instance`, `click_description`).
///
/// # Parameters
///
/// - `$name`: The wrapper struct name (e.g. `ClickBinding`).
/// - `$kind`: An `ActionKind` variant (e.g. `ActionKind::Click`).
///
/// # Generated Struct
///
/// For `ActionKind::Click`, the macro generates:
///
/// ```rust
/// #[derive(Clone, Debug, Default, Deserialize)]
/// #[serde(default)]
/// pub struct ClickBinding {
///     #[serde(rename = "click_topic")]
///     pub topic: Option<String>,
///     #[serde(rename = "click_payload")]
///     pub payload: Option<Value>,
///     #[serde(rename = "click_instance")]
///     pub instance: Option<String>,
///     #[serde(rename = "click_description")]
///     pub description: Option<String>,
/// }
/// ```
macro_rules! action_binding {
    ($name:ident, $kind:expr) => {
        #[derive(Clone, Debug, Default, serde::Deserialize)]
        #[serde(default)]
        pub struct $name {
            #[serde(rename = concat!($kind.prefix(), "_topic"))]
            pub topic: Option<String>,
            #[serde(rename = concat!($kind.prefix(), "_payload"))]
            pub payload: Option<serde_json::Value>,
            #[serde(rename = concat!($kind.prefix(), "_instance"))]
            pub instance: Option<String>,
            #[serde(rename = concat!($kind.prefix(), "_description"))]
            pub description: Option<String>,
        }

        impl $name {
            /// Returns a reference to the inner `ActionBinding` fields.
            ///
            /// This provides access to the reusable `is_configured()`,
            /// `resolve()`, `dispatch()`, and `as_tool_description()` methods.
            pub fn as_binding(&self) -> ActionBinding {
                ActionBinding {
                    topic: self.topic.clone(),
                    payload: self.payload.clone(),
                    instance: self.instance.clone(),
                    description: self.description.clone(),
                }
            }
        }
    };
}
```

### 6.2 Why `#[serde(flatten)]` Works Here

`#[serde(flatten)]` with multiple wrapper structs works because each wrapper struct has **uniquely named fields** (via `#[serde(rename)]`). When serde
deserializes a flattened struct, it collects all remaining fields into a shared content buffer and each flattened struct extracts only its own renamed fields.
Since `ClickBinding` looks for `click_topic` and `LongpressBinding` looks for `longpress_topic`, there is no collision.

### 6.3 Pre-Generated Wrapper Structs

To avoid requiring every config struct to invoke the macro, the wrapper structs are generated once in `plugin-api/src/atomic/action.rs` and re-exported:

```rust
action_binding!(ClickBinding, ActionKind::Click);
action_binding!(LongpressBinding, ActionKind::Longpress);
action_binding!(HoldBinding, ActionKind::Hold);
action_binding!(DoublePressBinding, ActionKind::DoublePress);
action_binding!(SwipeUpBinding, ActionKind::SwipeUp);
action_binding!(SwipeDownBinding, ActionKind::SwipeDown);
action_binding!(CompoundLongpressBinding, ActionKind::CompoundLongpress);
action_binding!(InitBinding, ActionKind::Init);
```

---

## 7. Config Struct Migration

### 7.1 AtomicWidgetConfig (Before)

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AtomicWidgetConfig {
    pub click_topic: Option<String>,
    pub click_payload: Option<Value>,
    pub click_instance: Option<String>,
    pub longpress_topic: Option<String>,
    pub longpress_payload: Option<Value>,
    pub longpress_instance: Option<String>,
    pub hold_topic: Option<String>,
    pub hold_payload: Option<Value>,
    pub hold_instance: Option<String>,
    pub double_press_topic: Option<String>,
    pub double_press_payload: Option<Value>,
    pub double_press_instance: Option<String>,
    pub compound_longpress_topic: Option<String>,
    pub compound_longpress_payload: Option<Value>,
    pub compound_longpress_instance: Option<String>,
    pub description: Option<String>,
    pub render_mode: Option<AtomicRenderMode>,
    pub show_main_text: Option<bool>,
    pub show_info_text: Option<bool>,
    pub text_backdrop_opacity: Option<f32>,
}
```

### 7.2 AtomicWidgetConfig (After)

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AtomicWidgetConfig {
    #[serde(flatten)]
    pub click: ClickBinding,
    #[serde(flatten)]
    pub longpress: LongpressBinding,
    #[serde(flatten)]
    pub hold: HoldBinding,
    #[serde(flatten)]
    pub double_press: DoublePressBinding,
    #[serde(flatten)]
    pub compound_longpress: CompoundLongpressBinding,
    /// Optional description for MCP tool registration.
    pub description: Option<String>,
    pub render_mode: Option<AtomicRenderMode>,
    pub show_main_text: Option<bool>,
    pub show_info_text: Option<bool>,
    pub text_backdrop_opacity: Option<f32>,
}
```

**Field count**: 15 action fields → 5 flattened bindings. Each binding provides 4 fields (topic, payload, instance, description), so the effective field count
is 20 — but the **source code** is dramatically shorter and the `Default` impl is auto-derived.

### 7.3 ButtonConfig (After)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ButtonConfig {
    pub text: String,
    #[serde(default = "default_width")]
    pub width: i32,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub icon_only: bool,
    #[serde(flatten)]
    pub click: ClickBinding,
    #[serde(flatten)]
    pub longpress: LongpressBinding,
    #[serde(flatten)]
    pub hold: HoldBinding,
    #[serde(flatten)]
    pub double_press: DoublePressBinding,
    #[serde(flatten)]
    pub swipe_up: SwipeUpBinding,
    #[serde(flatten)]
    pub swipe_down: SwipeDownBinding,
    #[serde(flatten)]
    pub compound_longpress: CompoundLongpressBinding,
    #[serde(flatten)]
    pub init: InitBinding,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub press_animation: Option<String>,
    pub spacing: i32,
    pub css_classes: Vec<String>,
    pub label_topic: Option<String>,
    pub label_format: Option<String>,
    pub label_fallback: Option<String>,
    pub state_topic: Option<String>,
    pub state_icon: Option<String>,
    pub state_css_class: Option<String>,
    pub state_label: Option<String>,
    pub description: Option<String>,
}
```

### 7.4 TOML Config (Unchanged)

```toml
[shelly_livingroom_couch_floor_light_button]
text = "Living Room Couch Floor"
icon = "nf-md-lightbulb"
description = "Steuert das Couch-Boden-Licht im Wohnzimmer"
click_topic = "service.http.request"
click_payload = { method = "Get", url = "http://192.168.178.25/color/0?turn=on&red=255&green=0&blue=0&gain=100", response_topic = "service.http.response.shelly.livingroom.couch.floor.light" }
click_description = "Schaltet das Licht ein (rot)"
longpress_topic = "service.http.request"
longpress_payload = { method = "Get", url = "http://192.168.178.25/color/0?turn=off", response_topic = "service.http.response.shelly.livingroom.couch.floor.light" }
longpress_description = "Schaltet das Licht aus"
swipe_up_topic = "service.http.request"
swipe_up_payload = { method = "Get", url = "http://192.168.178.25/color/0?turn=on&red=255&green=0&blue=0&gain={gain+10}", response_topic = "service.http.response.shelly.livingroom.couch.floor.light" }
swipe_up_description = "Erhöht die Helligkeit"
swipe_down_topic = "service.http.request"
swipe_down_payload = { method = "Get", url = "http://192.168.178.25/color/0?turn=on&red=255&green=0&blue=0&gain={gain-10}", response_topic = "service.http.response.shelly.livingroom.couch.floor.light" }
swipe_down_description = "Verringert die Helligkeit"
```

The TOML config format remains **100% backward compatible**. No user-facing migration is required.

---

## 8. Code Access Changes

### 8.1 Field Access (Before → After)

| Before                          | After                           |
|---------------------------------|---------------------------------|
| `self.config.click_topic`       | `self.config.click.topic`       |
| `self.config.click_payload`     | `self.config.click.payload`     |
| `self.config.click_instance`    | `self.config.click.instance`    |
| `self.config.click_description` | `self.config.click.description` |
| `self.config.longpress_topic`   | `self.config.longpress.topic`   |
| `self.config.swipe_up_payload`  | `self.config.swipe_up.payload`  |
| `self.config.init_instance`     | `self.config.init.instance`     |

### 8.2 Dispatch Logic (Before → After)

**Before** (`AtomicWidgetConfig::resolve_action`):

```rust
pub fn resolve_action(&self, action: AtomicAction) -> Option<ResolvedAction<'_>> {
    let (topic, payload, instance) = match action {
        AtomicAction::Click => (&self.click_topic, &self.click_payload, &self.click_instance),
        AtomicAction::Longpress => (&self.longpress_topic, &self.longpress_payload, &self.longpress_instance),
        // ...
    };
    let topic = topic.as_ref().map(|s| s.as_str())?;
    let payload = payload.as_ref()?;
    let instance = instance.as_ref();
    Some(ResolvedAction { topic, payload, instance })
}
```

**After** (`AtomicWidgetConfig::resolve_action`):

```rust
pub fn resolve_action(&self, action: AtomicAction) -> Option<ResolvedAction<'_>> {
    let binding = match action {
        AtomicAction::Click => &self.click,
        AtomicAction::Longpress => &self.longpress,
        AtomicAction::HoldStart | AtomicAction::HoldStop => &self.hold,
        AtomicAction::DoublePress => &self.double_press,
        AtomicAction::CompoundLongpress => &self.compound_longpress,
    };
    binding.as_binding().resolve()
}
```

**After** (`AtomicWidgetConfig::dispatch_action`):

```rust
pub fn dispatch_action(&self, broadcaster: &MessageBroadcasterInner, action: AtomicAction) {
    let binding = match action {
        AtomicAction::Click => &self.click,
        AtomicAction::Longpress => &self.longpress,
        AtomicAction::HoldStart | AtomicAction::HoldStop => &self.hold,
        AtomicAction::DoublePress => &self.double_press,
        AtomicAction::CompoundLongpress => &self.compound_longpress,
    };
    binding.as_binding().dispatch(broadcaster);
}
```

### 8.3 ButtonWidget Gesture Handlers (After)

The `build_widget` method in `plugins/button/src/widget.rs` currently clones `click_topic`, `click_payload`, `click_instance` separately into closures. After
the refactor, the entire `ActionBinding` is cloned as one unit:

```rust
let click_binding = self .config.click.as_binding();
let message_broadcaster = self .get_broadcaster();
button.connect_clicked(move | _ | {
click_binding.dispatch( & message_broadcaster);
});
```

This is cleaner and less error-prone than cloning three separate `Option`s.

---

## 9. Voice Assistant Integration

### 9.1 Per-Action Descriptions

The `description` field on each `ActionBinding` provides the Voice Assistant LLM with context about what each action does. This is critical for tool calling
accuracy:

- **Without descriptions**: The LLM sees a tool `button_livingroom_light` with an `action` parameter (`"click"` / `"longpress"` / `"swipe_up"` / ...) but has no
  idea what each action does. It must guess based on the button's `text` label.
- **With descriptions**: The LLM sees per-action descriptions like `"Schaltet das Licht ein (rot)"` and `"Schaltet das Licht aus"`, enabling precise tool
  selection.

### 9.2 MCP Tool Registration

The `atomic_widget_impl!` macro and `ButtonWidget::register_mcp_capabilities()` can now build a richer tool schema that includes per-action descriptions:

```json
{
  "type": "object",
  "properties": {
    "action": {
      "type": "string",
      "enum": ["click", "longpress", "hold_start", "hold_stop", "double_press", "swipe_up", "swipe_down", "compound_longpress"],
      "description": "The action to trigger",
      "action_descriptions": {
        "click": "Schaltet das Licht ein (rot)",
        "longpress": "Schaltet das Licht aus",
        "swipe_up": "Erhöht die Helligkeit",
        "swipe_down": "Verringert die Helligkeit"
      }
    }
  },
  "required": ["action"]
}
```

This is a future enhancement. The immediate refactor ensures the `description` field is available on all action kinds; the MCP tool schema can be enriched in a
follow-up phase.

---

## 10. Affected Config Structs

| Config Struct             | Crate                        | Action Kinds Used                                                                |
|---------------------------|------------------------------|----------------------------------------------------------------------------------|
| `AtomicWidgetConfig`      | `plugin-api`                 | Click, Longpress, Hold, DoublePress, CompoundLongpress                           |
| `ButtonConfig`            | `plugins/button`             | Click, Longpress, Hold, DoublePress, SwipeUp, SwipeDown, CompoundLongpress, Init |
| `AppLauncherConfig`       | `plugins/app-launcher`       | Click, Longpress                                                                 |
| `AudioWidgetConfig`       | `plugins/audio`              | Click, Longpress                                                                 |
| `ClockConfig`             | `plugins/clock`              | Click, Longpress                                                                 |
| `MprisWidgetConfig`       | `plugins/mpris`              | Click, Longpress                                                                 |
| `NetworkWidgetConfig`     | `plugins/network`            | Click, Longpress                                                                 |
| `PowerWidgetConfig`       | `plugins/power`              | Click, Longpress                                                                 |
| `WallpaperWidgetConfig`   | `plugins/wallpaper`          | Click, Longpress                                                                 |
| `WeatherWidgetConfig`     | `plugins/weather`            | Click, Longpress                                                                 |
| `WorkspaceSwitcherConfig` | `plugins/workspace-switcher` | Click, Longpress                                                                 |

**Total**: 11 config structs, 8 action kinds, ~30 field triplets to refactor.

---

## 11. Migration Plan

The migration is organised into **five phases**. Each phase is independently buildable and testable — the project remains in a working state after each phase.

---

### Phase 1: Foundation — ActionKind, ActionBinding, Wrapper Structs

**Goal**: Add `ActionKind`, `ActionBinding`, and all wrapper structs to `plugin-api`. No existing config struct is modified yet. All widgets continue to use
their current field-based config.

**Steps**:

1. **Add `ActionKind` enum** to `plugin-api/src/atomic/action.rs`:
    - Enum with variants: `Click`, `Longpress`, `Hold`, `DoublePress`, `SwipeUp`, `SwipeDown`, `CompoundLongpress`, `Init`.
    - Derive `Clone`, `Copy`, `Debug`, `Eq`, `PartialEq`, `Hash`.
    - Implement `prefix()` method returning the TOML field prefix string.

2. **Add `ActionBinding` struct** to `plugin-api/src/atomic/action.rs`:
    - Fields: `topic: Option<String>`, `payload: Option<Value>`, `instance: Option<String>`, `description: Option<String>`.
    - Derive `Clone`, `Debug`, `Default`, `Deserialize`.
    - Implement `is_configured()`, `resolve()`, `dispatch()`, `as_tool_description()`.

3. **Add `action_binding!` declarative macro** to `plugin-api/src/atomic/action.rs`:
    - Macro accepting `$name:ident` and `$kind:expr` (an `ActionKind` variant).
    - Generates wrapper struct with `#[serde(rename = concat!($kind.prefix(), "_topic"))]` etc.

4. **Generate all eight wrapper structs** in `plugin-api/src/atomic/action.rs`:
    - `ClickBinding`, `LongpressBinding`, `HoldBinding`, `DoublePressBinding`, `SwipeUpBinding`, `SwipeDownBinding`, `CompoundLongpressBinding`, `InitBinding`.
    - Each wrapper has an `as_binding()` method returning an owned `ActionBinding`.

5. **Update `plugin-api/src/atomic/mod.rs`**:
    - Re-export `ActionKind`, `ActionBinding`, all wrapper structs, and the `action_binding!` macro.

**Verification**: `cargo build -p smearor-swipe-launcher-plugin-api` succeeds. No widget behaviour changes.

**Files changed**:

| File                              | Change                                                                      |
|-----------------------------------|-----------------------------------------------------------------------------|
| `plugin-api/src/atomic/action.rs` | Add `ActionKind`, `ActionBinding`, `action_binding!` macro, wrapper structs |
| `plugin-api/src/atomic/mod.rs`    | Re-export new types and macro                                               |

---

### Phase 2: Migrate AtomicWidgetConfig

**Goal**: Refactor `AtomicWidgetConfig` to use flattened wrapper structs. Update `resolve_action()` and `dispatch_action()`. Update the `atomic_widget_impl!`
macro to use the new field access pattern.

**Steps**:

1. **Replace action fields** in `AtomicWidgetConfig` with flattened wrapper structs:
    - `click: ClickBinding`, `longpress: LongpressBinding`, `hold: HoldBinding`, `double_press: DoublePressBinding`,
      `compound_longpress: CompoundLongpressBinding`.
    - Remove the `Default` impl — `#[serde(default)]` on each flattened struct handles defaults.

2. **Simplify `resolve_action()`**:
    - Match on `AtomicAction`, return `binding.as_binding().resolve()`.

3. **Simplify `dispatch_action()`**:
    - Match on `AtomicAction`, call `binding.as_binding().dispatch(broadcaster)`.

4. **Update `atomic_widget_impl!` macro** in `plugin-api/src/atomic/macro.rs`:
    - Change `self.config.click_topic` → `self.config.click.topic` etc.
    - Update `register_mcp_capabilities()` to use `self.config.click.description` etc.

**Verification**: `cargo build` succeeds for `plugin-api` and all atomic widget crates (audio, mpris, weather). `cargo test` passes. TOML configs parse
correctly.

**Files changed**:

| File                              | Change                                                   |
|-----------------------------------|----------------------------------------------------------|
| `plugin-api/src/atomic/config.rs` | Replace fields with flattened bindings, simplify methods |
| `plugin-api/src/atomic/macro.rs`  | Update field access patterns                             |

---

### Phase 3: Migrate ButtonConfig

**Goal**: Refactor `ButtonConfig` to use all eight wrapper structs. Update `ButtonWidget` gesture handlers and `handle_message` to use
`ActionBinding::dispatch()`.

**Steps**:

1. **Replace action fields** in `ButtonConfig` with flattened wrapper structs:
    - `click`, `longpress`, `hold`, `double_press`, `swipe_up`, `swipe_down`, `compound_longpress`, `init`.
    - Remove manual `Default` impl for action fields (handled by `#[serde(default)]` on each binding).

2. **Update `ButtonWidget::new()`** in `plugins/button/src/widget.rs`:
    - Change `config.init_topic` → `config.init.topic` etc.

3. **Update `ButtonWidget::handle_message()`**:
    - Replace the large match block with `binding.as_binding().dispatch(&broadcaster)`.

4. **Update `ButtonWidget::build_widget()`**:
    - Replace separate `click_topic`/`click_payload`/`click_instance` clones with `self.config.click.as_binding()`.
    - Same for `longpress`, `swipe_up`, `swipe_down`.
    - Use `ActionBinding::dispatch()` inside closures instead of manual broadcast logic.

5. **Update `ButtonWidget::register_mcp_capabilities()`**:
    - Use `self.config.click.description` etc. for per-action descriptions in the tool schema.

**Verification**: `cargo build -p smearor-button-widget` succeeds. `cargo test` passes. Button gestures (click, longpress, swipe, hold, double press) work
correctly.

**Files changed**:

| File                           | Change                                                   |
|--------------------------------|----------------------------------------------------------|
| `plugins/button/src/config.rs` | Replace fields with flattened bindings                   |
| `plugins/button/src/widget.rs` | Update all field access, use `ActionBinding::dispatch()` |

---

### Phase 4: Migrate Remaining Config Structs

**Goal**: Refactor all remaining config structs to use `ClickBinding` and `LongpressBinding` (and `InitBinding` where applicable). This phase adds missing
`instance` and `description` fields to structs that previously lacked them.

**Steps**:

1. **Migrate `AppLauncherConfig`** (`plugins/app-launcher/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with `click: ClickBinding`, `longpress: LongpressBinding`.
    - Update `plugins/app-launcher/src/widget.rs` field access.

2. **Migrate `AudioWidgetConfig`** (`plugins/audio/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with flattened bindings.
    - **Adds**: `click_instance`, `click_description`, `longpress_instance`, `longpress_description` (previously missing).
    - Update `plugins/audio/src/widget.rs` field access.

3. **Migrate `ClockConfig`** (`plugins/clock/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with flattened bindings.
    - **Adds**: `click_instance`, `longpress_instance` (previously missing).
    - Update `plugins/clock/src/widget.rs` or `clock.rs` field access.

4. **Migrate `MprisWidgetConfig`** (`plugins/mpris/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with flattened bindings.
    - **Adds**: `click_instance`, `click_description`, `longpress_instance`, `longpress_description` (previously missing).
    - Update `plugins/mpris/src/widget.rs` field access.

5. **Migrate `NetworkWidgetConfig`** (`plugins/network/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with flattened bindings.
    - **Adds**: `click_description`, `longpress_description` (previously missing).
    - Update `plugins/network/src/widget.rs` field access.

6. **Migrate `PowerWidgetConfig`** (`plugins/power/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with flattened bindings.
    - Update `plugins/power/src/widget.rs` field access.

7. **Migrate `WallpaperWidgetConfig`** (`plugins/wallpaper/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with flattened bindings.
    - **Adds**: `click_description`, `longpress_description` (previously missing).
    - Update `plugins/wallpaper/src/widget.rs` field access.

8. **Migrate `WeatherWidgetConfig`** (`plugins/weather/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with flattened bindings.
    - **Adds**: `click_instance`, `click_description`, `longpress_instance`, `longpress_description` (previously missing).
    - Update `plugins/weather/src/widget.rs` field access.

9. **Migrate `WorkspaceSwitcherConfig`** (`plugins/workspace-switcher/src/config.rs`):
    - Replace `click_*` / `longpress_*` fields with flattened bindings.
    - **Adds**: `click_instance`, `longpress_instance` (previously missing).
    - Update `plugins/workspace-switcher/src/widget.rs` field access.

**Verification**: `cargo build` succeeds for all plugin crates. `cargo test` passes. All TOML configs parse correctly. Widgets that previously lacked `instance`
or `description` fields now accept them in TOML config.

**Files changed**:

| File                                       | Change                                           |
|--------------------------------------------|--------------------------------------------------|
| `plugins/app-launcher/src/config.rs`       | Replace fields with flattened bindings           |
| `plugins/app-launcher/src/widget.rs`       | Update field access                              |
| `plugins/audio/src/config.rs`              | Replace fields, add missing instance/description |
| `plugins/audio/src/widget.rs`              | Update field access                              |
| `plugins/clock/src/config.rs`              | Replace fields, add missing instance             |
| `plugins/clock/src/widget.rs`              | Update field access                              |
| `plugins/mpris/src/config.rs`              | Replace fields, add missing instance/description |
| `plugins/mpris/src/widget.rs`              | Update field access                              |
| `plugins/network/src/config.rs`            | Replace fields, add missing description          |
| `plugins/network/src/widget.rs`            | Update field access                              |
| `plugins/power/src/config.rs`              | Replace fields with flattened bindings           |
| `plugins/power/src/widget.rs`              | Update field access                              |
| `plugins/wallpaper/src/config.rs`          | Replace fields, add missing description          |
| `plugins/wallpaper/src/widget.rs`          | Update field access                              |
| `plugins/weather/src/config.rs`            | Replace fields, add missing instance/description |
| `plugins/weather/src/widget.rs`            | Update field access                              |
| `plugins/workspace-switcher/src/config.rs` | Replace fields, add missing instance             |
| `plugins/workspace-switcher/src/widget.rs` | Update field access                              |

---

### Phase 5: Tests and Documentation

**Goal**: Add unit tests for `ActionBinding`, `ActionKind`, and the wrapper structs. Update documentation.

**Steps**:

1. **Add unit tests** in `plugin-api/src/atomic/action.rs`:
    - Test `ActionBinding::is_configured()` with various field combinations.
    - Test `ActionBinding::resolve()` returns `Some` when configured, `None` when not.
    - Test `ActionKind::prefix()` for all variants.
    - Test TOML deserialization: a config with `click_topic = "..."` deserializes into `ClickBinding { topic: Some("..."), ... }`.
    - Test TOML deserialization: a config without any action fields deserializes to all-`None` bindings.
    - Test `#[serde(flatten)]` with multiple bindings: `click_topic` and `longpress_topic` do not collide.

2. **Add integration tests** in `plugins/button/src/config.rs`:
    - Test `ButtonConfig` deserialization with all action kinds.
    - Test that `click_description` and `swipe_up_description` are parsed correctly.

3. **Update documentation**:
    - Document `ActionBinding` and `ActionKind` with rustdoc comments.
    - Document the `action_binding!` macro with an example.
    - Update `AGENTS.md` if needed to reference the new pattern.

**Verification**: `cargo test` passes for all crates. `cargo doc` generates clean documentation.

**Files changed**:

| File                              | Change                |
|-----------------------------------|-----------------------|
| `plugin-api/src/atomic/action.rs` | Add unit tests        |
| `plugins/button/src/config.rs`    | Add integration tests |

---

### Phase Dependency Graph

```
Phase 1 (Foundation)
    │
    ▼
Phase 2 (AtomicWidgetConfig)
    │
    ├──▶ Phase 3 (ButtonConfig) — can proceed in parallel with Phase 4
    │
    └──▶ Phase 4 (Remaining Configs) — can proceed per-widget
              │
              ▼
         Phase 5 (Tests & Docs)
```

Phase 1 is the prerequisite for all other phases. Phase 2 and Phase 3 can proceed in parallel after Phase 1. Phase 4 can proceed per-widget after Phase 2 (since
it follows the same pattern). Phase 5 is the final phase.

---

## 12. Resolved Design Decisions

- **Struct name**: `ActionBinding` — clear, concise, and describes what it is (a binding for an action). The wrapper structs are named `<Kind>Binding` (e.g.
  `ClickBinding`, `LongpressBinding`).

- **Enum vs string literals in macro**: The macro uses `ActionKind` enum variants instead of string literals. This provides compile-time safety: a typo in
  `ActionKind::Clikc` is a compile error, while a typo in `"clikc"` would silently generate wrong `#[serde(rename)]` attributes. The `prefix()` method on
  `ActionKind` is the single source of truth for the TOML field prefix.

- **`as_binding()` returns owned `ActionBinding`**: The wrapper structs store the same fields as `ActionBinding` but cannot directly deref to `ActionBinding`
  because they have different struct layouts (due to `#[serde(rename)]`). The `as_binding()` method clones the fields into an owned `ActionBinding`. This is a
  cheap operation (four `Option` clones). Alternatively, a `Deref` impl could return a reference, but this would require the wrapper struct to contain an
  `ActionBinding` field rather than individual fields — which conflicts with `#[serde(flatten)]` + `#[serde(rename)]`. The owned-clone approach is simpler and
  performance is irrelevant here (config is parsed once at startup).

- **`#[serde(flatten)]` with multiple wrappers**: This works because each wrapper struct has uniquely renamed fields. serde's flatten implementation collects
  all remaining fields into a content buffer and each flattened struct extracts only its own fields by name. No collision occurs because `click_topic` and
  `longpress_topic` are different names.

- **Adding missing `instance` and `description` fields**: This is a **non-breaking change** for TOML configs. Existing configs that do not use `*_instance` or
  `*_description` continue to work (the fields default to `None`). Configs that previously could not target specific instances or provide per-action
  descriptions now can.

- **`description` field importance for Voice Assistant**: The per-action `description` field is critical for LLM tool calling. When the Voice Assistant receives
  a tool invocation request, it needs to know which action to select. Without per-action descriptions, the LLM only sees the button's top-level `description`
  and the generic `action` enum. With per-action descriptions, the LLM can make informed decisions: `"click_description = Schaltet das Licht ein (rot)"` vs
  `"longpress_description = Schaltet das Licht aus"`. This is especially important for buttons with many actions (click, longpress, swipe_up, swipe_down) where
  the semantic difference between actions is not obvious from the button label alone.

- **Top-level `description` vs per-action `description`**: The top-level `description` field on config structs is used for MCP tool registration (the tool's
  overall description). Per-action `description` fields describe individual actions within that tool. Both serve different purposes and coexist. The top-level
  description says "this button controls the living room light"; the per-action descriptions say "click turns it on (red)", "longpress turns it off", "swipe_up
  increases brightness".
