# Using Action Bindings

Action bindings allow widgets to dispatch broker messages when the user interacts with them. The `ActionBindings` struct from `plugin-api` provides a unified
configuration for all interaction types.

## ActionBindings Struct

```rust
pub struct ActionBindings {
    pub click: Option<ClickBinding>,
    pub longpress: Option<LongpressBinding>,
    pub hold: Option<HoldBinding>,
    pub double_press: Option<DoublePressBinding>,
    pub swipe_up: Option<SwipeUpBinding>,
    pub swipe_down: Option<SwipeDownBinding>,
    pub right_click: Option<RightClickBinding>,
    pub middle_click: Option<MiddleClickBinding>,
    pub scroll_up: Option<ScrollUpBinding>,
    pub scroll_down: Option<ScrollDownBinding>,
    pub compound_longpress: Option<CompoundLongpressBinding>,
    pub init: Option<InitBinding>,
}
```

## Embedding in Widget Config

Use `#[serde(flatten)]` to embed `ActionBindings` into your widget's config:

```rust
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct MyWidgetConfig {
    pub main_text: String,
    #[serde(flatten)]
    pub action_bindings: ActionBindings,
}
```

## Binding Structure

Each binding has:

- `topic` — Broker topic to send to
- `payload` — TOML inline table (serialized to JSON string)
- `instance` — Optional target instance ID
- `mode` — `BindingMode::Replace` or `BindingMode::Supplement`

## Dispatching Actions

Use the `DispatchableBinding` trait to dispatch a binding:

```rust
use smearor_swipe_launcher_plugin_api::DispatchableBinding;

if let Some(ref click) = self.config.action_bindings.click {
    click.dispatch(self, &self.config.action_bindings);
}
```

## Default Fallback

Implement `DefaultFallback` to define what happens when no binding is configured:

```rust
use smearor_swipe_launcher_plugin_api::DefaultFallback;

impl DefaultFallback for MyWidget {
    fn default_fallback(&self, kind: &ActionKind) {
        match kind {
            ActionKind::Click => {
                // Default click behavior
            }
            _ => {}
        }
    }
}
```

## Binding Modes

- **`Replace`** — Only the configured binding is dispatched; the default fallback is skipped
- **`Supplement`** — Both the binding **and** the default fallback are dispatched

## ActionKind

The `ActionKind` enum identifies which interaction triggered:

```rust
pub enum ActionKind {
    Click,
    Longpress,
    Hold,
    HoldStart,
    HoldStop,
    DoublePress,
    SwipeUp,
    SwipeDown,
    RightClick,
    MiddleClick,
    ScrollUp,
    ScrollDown,
    CompoundLongpress,
    Init,
}
```

## Gesture Handler Integration

Widgets that use `attach_gesture_handlers` automatically get gesture detection. The gesture handler calls the appropriate binding based on the detected
interaction.

See [Action Bindings](../features/action-bindings.md) for the feature perspective and configuration format.
