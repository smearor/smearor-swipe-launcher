# Button Widget Internal State

This document describes the concept for adding **internal state** to the Button widget. The internal state is a JSON value that can be updated by subscribing to
a topic and can be used to drive visual properties of the button (icon, CSS classes, label). It also introduces an **initial one-shot request** mechanism so the
button can query the current state from a service on startup.

The primary example and test case is the **Shelly Fan** button, which should reflect whether the fan is currently on or off.

---

## 1. Motivation

Currently the Button widget supports:

- **Click** – sends a message on click
- **Long-press** – sends a message on long-press
- **Swipe-up / Swipe-down** – sends a message on swipe gestures
- **Dynamic label** – subscribes to a `label_topic` and updates the label text

What is missing is the ability to reflect **external state** in the button's visual appearance. For example:

- A fan button that shows a different icon depending on whether the fan is on or off
- A light button that adds a CSS class when the light is active
- A button whose label, icon, and styling all depend on a JSON state received from a service

The existing `label_topic` mechanism is too narrow: it only updates the label text. The internal state feature generalizes this to a full JSON state that can
drive multiple visual properties.

---

## 2. Use Case: Shelly Fan

The Shelly Fan button currently sends HTTP requests via the HTTP service to turn the fan on or off:

```toml
[shelly_fan_button]
text = "Fan"
icon = "nf-md-fan"
icon_only = true
click_topic = "service.http.request"
click_payload = { method = "Get", url = "http://192.168.178.39/relay/0?turn=on", response_topic = "service.http.response.shelly.fan" }
longpress_topic = "service.http.request"
longpress_payload = { method = "Get", url = "http://192.168.178.39/relay/0?turn=off", response_topic = "service.http.response.shelly.fan" }
```

The HTTP service publishes the response to `service.http.response.shelly.fan`. The response JSON from a Shelly 1 relay looks like:

```json
{
  "ison": true,
  "has_timer": false,
  "timer_started": 0,
  "timer_duration": 0,
  "timer_remaining": 0,
  "overpower": false,
  "source": "http"
}
```

The goal: the button should subscribe to `service.http.response.shelly.fan`, store the response as internal state, and visually reflect the `ison` field.

---

## 3. Configuration

### 3.1 New Config Fields

The following fields are added to `ButtonConfig`:

```rust
/// Topic whose messages update the internal state (JSON)
#[serde(default)]
pub state_topic: Option<String>,
/// Initial one-shot request topic (sent on widget construction)
#[serde(default)]
pub init_topic: Option<String>,
/// Initial one-shot request payload (JSON/TOML)
#[serde(default)]
pub init_payload: Option<Value>,
/// Target instance for the initial one-shot request
#[serde(default)]
pub init_instance: Option<String>,
```

### 3.2 State-Driven Visual Configuration

The internal state JSON can drive the button's appearance through template expressions. The following fields use JSON path expressions that are resolved against
the internal state:

```rust
/// Icon expression evaluated against the internal state.
/// Supports static icon names and JSON path expressions like "{ison}" or
/// conditional mappings like "{ison?nf-md-fan:nf-md-fan-off}".
#[serde(default)]
pub state_icon: Option<String>,
/// CSS class expression evaluated against the internal state.
/// Adds the resolved class to the button when the expression is truthy,
/// removes it when falsy. Example: "fan-active" adds the class when
/// the state is truthy.
#[serde(default)]
pub state_css_class: Option<String>,
/// Label expression evaluated against the internal state.
/// Uses the same format string syntax as label_format.
#[serde(default)]
pub state_label: Option<String>,
```

### 3.3 Example Configuration: Shelly Fan

```toml
[shelly_fan_button]
text = "Fan"
icon = "nf-md-fan"
icon_only = true

# Click: turn fan on
click_topic = "service.http.request"
click_payload = { method = "Get", url = "http://192.168.178.39/relay/0?turn=on", response_topic = "service.http.response.shelly.fan" }

# Long-press: turn fan off
longpress_topic = "service.http.request"
longpress_payload = { method = "Get", url = "http://192.168.178.39/relay/0?turn=off", response_topic = "service.http.response.shelly.fan" }

# Subscribe to HTTP response to track fan state
state_topic = "service.http.response.shelly.fan"

# On startup, query the current fan state
init_topic = "service.http.request"
init_payload = { method = "Get", url = "http://192.168.178.39/relay/0", response_topic = "service.http.response.shelly.fan" }

# Icon changes based on ison field
state_icon = "{ison?nf-md-fan:nf-md-fan-off}"

# CSS class when fan is running
state_css_class = "fan-active"

# Label text (visible when icon_only = false)
state_label = "Fan: {ison}"
```

---

## 4. Architecture

### 4.1 Message Flow

```
+------------------+         +------------------+         +------------------+
| Button Widget    |         | HTTP Service     |         | Shelly Device    |
|                  |         | (Singleton)      |         |                  |
| 1. On construct: |========>| 2. Execute HTTP  |========>| 3. Return state  |
|    send init     |         |    request       |         |    JSON          |
|                  |         |                  |         |                  |
| 5. Update state  |<========| 4. Publish       |<========|                  |
|    from response |         |    response to   |         |                  |
|    topic         |         |    response_topic|         |                  |
|                  |         |                  |         |                  |
| 6. On click:     |========>| 7. Execute HTTP  |========>| 8. Toggle relay  |
|    send command  |         |    toggle request|         |                  |
|                  |         |                  |         |                  |
| 9. Update state  |<========| 10. Publish      |<========|                  |
|    from response |         |     response     |         |                  |
+------------------+         +------------------+         +------------------+
```

### 4.2 Internal State Lifecycle

```
Widget Construction
        |
        v
+-------------------+     +-------------------+     +-------------------+
| Send init one-shot|---->| Wait for state    |---->| Update internal   |
| request           |     | topic message     |     | state (JSON)      |
+-------------------+     +-------------------+     +-------------------+
                                                          |
                                                          v
                                                  +-------------------+
                                                  | Re-evaluate      |
                                                  | state_icon,      |
                                                  | state_css_class, |
                                                  | state_label      |
                                                  +-------------------+
                                                          |
                                                          v
                                                  +-------------------+
                                                  | Update GTK widget |
                                                  | (icon, CSS, label)|
                                                  +-------------------+
```

### 4.3 State Storage

The internal state is stored as a `serde_json::Value` inside the widget struct:

```rust
pub struct ButtonWidget {
    pub(crate) meta: PluginMeta,
    pub(crate) core_context: Option<FfiCoreContext>,
    pub(crate) config: ButtonConfig,
    pub(crate) label_widget: std::cell::RefCell<Option<Label>>,
    pub(crate) icon_widget: std::cell::RefCell<Option<Image>>,
    pub(crate) internal_state: std::cell::RefCell<Option<serde_json::Value>>,
}
```

The state is updated whenever a message arrives on the `state_topic`. The existing `label_topic` mechanism remains unchanged for backward compatibility.

---

## 5. Implementation

### 5.1 Config Changes (`config.rs`)

Add the new fields to `ButtonConfig`:

```rust
/// Topic whose messages update the internal state (JSON)
#[serde(default)]
pub state_topic: Option<String>,
/// Initial one-shot request topic (sent on widget construction)
#[serde(default)]
pub init_topic: Option<String>,
/// Initial one-shot request payload (JSON/TOML)
#[serde(default)]
pub init_payload: Option<Value>,
/// Target instance for the initial one-shot request
#[serde(default)]
pub init_instance: Option<String>,
/// Icon expression evaluated against the internal state
#[serde(default)]
pub state_icon: Option<String>,
/// CSS class expression evaluated against the internal state
#[serde(default)]
pub state_css_class: Option<String>,
/// Label expression evaluated against the internal state
#[serde(default)]
pub state_label: Option<String>,
```

### 5.2 Widget Construction (`widget.rs`)

In `ButtonWidget::new`, after construction, send the init one-shot request:

```rust
pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
    let meta = PluginMeta::try_from(&config)?;
    let config = ButtonConfig::parse(&config.config)
        .map_err(|e| PluginConstructionErrorWrapper::new(
            PluginConstructionError::FailedToParseWidgetConfig,
            e.to_string().into(),
        ))?;

    let widget = Self {
        meta,
        core_context,
        config: config.clone(),
        label_widget: std::cell::RefCell::new(None),
        icon_widget: std::cell::RefCell::new(None),
        internal_state: std::cell::RefCell::new(None),
    };

    // Send initial one-shot request
    if let (Some(topic), Some(payload)) = (&config.init_topic, &config.init_payload) {
        let broadcaster = widget.get_broadcaster();
        let payload_str = payload.to_string();
        if let Some(instance) = &config.init_instance {
            broadcaster.broadcast_string_to_instance(instance, topic, &payload_str);
        } else {
            broadcaster.broadcast_string(topic, &payload_str);
        }
    }

    Ok(widget)
}
```

This follows the same pattern used by the WorkspaceSwitcher widget, which sends a `WorkspaceSnapshotRequestMessage` in its constructor.

### 5.3 Topic Subscription

The `AcceptTopic` and `on_message` implementations must be extended to handle the `state_topic` in addition to the existing `label_topic`:

```rust
impl AcceptTopic<FfiEnvelope> for ButtonWidget {
    fn accept_topic(&self, topic: &str) -> bool {
        if let Some(label_topic) = &self.config.label_topic {
            if topic == label_topic {
                return true;
            }
        }
        if let Some(state_topic) = &self.config.state_topic {
            if topic == state_topic {
                return true;
            }
        }
        false
    }
}
```

In `on_message`, handle both topics:

```rust
impl Plugin for ButtonWidget {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            let topic = envelope.topic.to_string();
            let type_id = smearor_swipe_launcher_plugin_api::generate_type_id("std::string::String");

            if envelope.type_id == type_id && !envelope.payload.is_null() {
                let payload = &*(envelope.payload as *const String);

                if let Some(label_topic) = &self.config.label_topic {
                    if topic == *label_topic {
                        self.update_label_from_message(payload);
                    }
                }

                if let Some(state_topic) = &self.config.state_topic {
                    if topic == *state_topic {
                        self.update_internal_state(payload);
                    }
                }
            }
        }
    }
}
```

### 5.4 State Update and Visual Refresh

```rust
impl ButtonWidget {
    fn update_internal_state(&self, payload: &str) {
        let json: serde_json::Value = match serde_json::from_str(payload) {
            Ok(value) => value,
            Err(_) => return,
        };

        *self.internal_state.borrow_mut() = Some(json.clone());

        let state = json.clone();
        let config = self.config.clone();
        let icon_weak = self.icon_widget.borrow().as_ref().map(|icon| icon.downgrade());
        let label_weak = self.label_widget.borrow().as_ref().map(|label| label.downgrade());

        glib::MainContext::default().spawn_local(async move {
            // Update icon
            if let Some(icon_expr) = &config.state_icon {
                let resolved = resolve_state_expression(icon_expr, &state);
                if let Some(icon) = icon_weak.and_then(|weak| weak.upgrade()) {
                    update_icon(&icon, &resolved, &config);
                }
            }

            // Update CSS class
            if let Some(css_expr) = &config.state_css_class {
                let is_active = resolve_state_truthy(&state, css_expr);
                // CSS class management requires a reference to the button,
                // handled via a weak reference passed into the closure
            }

            // Update label
            if let Some(label_expr) = &config.state_label {
                let text = resolve_state_format(label_expr, &state);
                if let Some(label) = label_weak.and_then(|weak| weak.upgrade()) {
                    label.set_text(&text);
                }
            }
        });
    }
}
```

### 5.5 Expression Resolution

The `state_icon` field supports a conditional expression syntax:

- **Static:** `nf-md-fan` – always uses this icon
- **JSON path:** `{ison}` – uses the string value of the `ison` field as the icon name
- **Conditional:** `{ison?nf-md-fan:nf-md-fan-off}` – if `ison` is truthy, uses `nf-md-fan`, otherwise `nf-md-fan-off`

```rust
/// Resolve a state expression against the internal state JSON.
/// Supports: "literal", "{field}", "{field?true_value:false_value}"
fn resolve_state_expression(expr: &str, state: &serde_json::Value) -> String {
    if !expr.starts_with('{') {
        return expr.to_string();
    }

    let inner = &expr[1..expr.len().saturating_sub(1)];

    if let Some((condition, values)) = inner.split_once('?') {
        let (true_val, false_val) = values.split_once(':').unwrap_or((values, ""));
        if is_truthy(&state[condition]) {
            true_val.to_string()
        } else {
            false_val.to_string()
        }
    } else {
        state[inner]
            .as_str()
            .unwrap_or(expr)
            .to_string()
    }
}

/// Check if a JSON value is truthy
fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty() && s != "false" && s != "0",
        serde_json::Value::Null => false,
        _ => true,
    }
}
```

### 5.6 Icon Widget Reference

The `build_widget` method must store a reference to the icon `Image` widget so it can be updated later:

```rust
// In build_widget, after creating the icon:
* self .icon_widget.borrow_mut() = Some(icon);
```

When `state_icon` is configured, the icon widget must be stored even when `icon_only` is true, so the icon can be swapped dynamically.

---

## 6. Backward Compatibility

- All new fields are optional with `#[serde(default)]`
- The existing `label_topic` / `label_format` / `label_fallback` mechanism remains unchanged
- Buttons without `state_topic` behave exactly as before
- The init one-shot is only sent when `init_topic` is configured
- No model crate changes are needed (the state is internal to the widget)

---

## 7. Relationship to Existing Patterns

| Pattern                  | Existing Example                                                         | This Feature                                               |
|--------------------------|--------------------------------------------------------------------------|------------------------------------------------------------|
| Initial one-shot request | WorkspaceSwitcher sends `WorkspaceSnapshotRequestMessage` in constructor | Button sends `init_payload` to `init_topic` in constructor |
| Topic subscription       | `label_topic` updates label text                                         | `state_topic` updates internal JSON state                  |
| State-driven UI          | Audio widget updates icon/label from `AudioStatusMessage`                | Button updates icon/CSS/label from internal JSON state     |
| HTTP response feedback   | Shelly buttons use `response_topic` for HTTP responses                   | Same `response_topic` is used as `state_topic`             |

---

## 8. Limitations and Future Extensions

### 8.1 Current Limitations

- **Single state topic:** Only one `state_topic` can be subscribed. Multiple topics would require a merge strategy.
- **No state persistence:** The internal state is lost on restart. The init one-shot mitigates this by re-querying on startup.
- **CSS class is binary:** `state_css_class` is either added or removed. Multiple CSS classes or conditional classes are not supported yet.
- **No state-driven click payload:** The click payload is static. A future extension could template the click payload against the internal state (e.g., toggle
  behavior).

### 8.2 Future Extensions

- **State-driven click payload:** Allow `click_payload` to be a template expression evaluated against the internal state, enabling toggle behavior (on -> send
  off command, off -> send on command).
- **Multiple state topics:** Subscribe to multiple topics and merge them into the internal state.
- **State persistence:** Persist the internal state across restarts using a config file or service.
- **State transitions:** Animate icon or CSS transitions when the state changes.
- **State-driven long-press payload:** Same templating as click payload.

---

## 9. Implementation Phases

### Phase 1: Config and State Storage

- Add `state_topic`, `init_topic`, `init_payload`, `init_instance` to `ButtonConfig`
- Add `internal_state` and `icon_widget` fields to `ButtonWidget`
- Send init one-shot in constructor
- Subscribe to `state_topic` in `AcceptTopic` and `on_message`
- Store received JSON in `internal_state`

**Exit criteria:** Button subscribes to a topic, stores JSON state internally, and logs the received state via `debug!`.

### Phase 2: State-Driven Icon

- Add `state_icon` config field
- Store icon widget reference in `build_widget`
- Implement `resolve_state_expression` for conditional icon names
- Update icon on state change

**Exit criteria:** Shelly Fan button shows `nf-md-fan` when on, `nf-md-fan-off` when off.

### Phase 3: State-Driven CSS Class

- Add `state_css_class` config field
- Add/remove CSS class based on truthiness of internal state
- Requires passing a button weak reference into the state update closure

**Exit criteria:** Shelly Fan button adds `fan-active` CSS class when `ison` is true.

### Phase 4: State-Driven Label

- Add `state_label` config field
- Reuse existing `label_format` resolution logic
- Update label on state change

**Exit criteria:** Shelly Fan button shows "Fan: true" when `icon_only = false`.

### Phase 5: Documentation and Testing

- Update `concepts/BUTTON_WIDGET.md` with new config parameters
- Test with Shelly Fan configuration
- Verify init one-shot works on startup
- Verify state updates on click and long-press responses

**Exit criteria:** Full Shelly Fan example works end-to-end with visual feedback.
