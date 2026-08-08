//! Action types, bindings, and dispatch for widget input triggers.
//!
//! This module is shared across all widget types (atomic and non-atomic)
//! and defines `ActionKind`, `ActionBinding`, wrapper structs, and the
//! `DispatchableBinding` trait.

use std::str::FromStr;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::MessageBroadcasterInner;

/// All possible input trigger kinds for widget actions.
///
/// Used by the `action_binding!` macro to generate per-kind wrapper structs
/// with correct `#[serde(rename)]` attributes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// Single-click action (press duration < 500 ms, fires on release).
    #[default]
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
    /// Right-click action (mouse only, secondary button).
    RightClick,
    /// Middle-click action (mouse only, middle button).
    MiddleClick,
    /// Scroll-up action (mouse wheel only).
    ScrollUp,
    /// Scroll-down action (mouse wheel only).
    ScrollDown,
    /// Compound longpress (two+ buttons in same span group held >= 500 ms).
    CompoundLongpress,
    /// Initial one-shot request (sent on widget construction).
    Init,
    /// Expand widget to expanded view (MCP tool action).
    Expand,
    /// Collapse widget to compact view (MCP tool action).
    Collapse,
    /// Toggle between compact and expanded views (MCP tool action).
    ToggleView,
}

/// Error returned when parsing an unknown action kind string.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("unknown action kind")]
pub struct UnknownActionKindError;

impl FromStr for ActionKind {
    type Err = UnknownActionKindError;

    /// Parses an action kind from its string representation.
    ///
    /// Both `"hold_start"` and `"hold_stop"` map to `ActionKind::Hold`.
    /// Returns `Err` if the string does not match a known action kind.
    fn from_str(action: &str) -> Result<Self, Self::Err> {
        match action {
            "click" => Ok(Self::Click),
            "longpress" => Ok(Self::Longpress),
            "hold_start" | "hold_stop" => Ok(Self::Hold),
            "double_press" => Ok(Self::DoublePress),
            "swipe_up" => Ok(Self::SwipeUp),
            "swipe_down" => Ok(Self::SwipeDown),
            "right_click" => Ok(Self::RightClick),
            "middle_click" => Ok(Self::MiddleClick),
            "scroll_up" => Ok(Self::ScrollUp),
            "scroll_down" => Ok(Self::ScrollDown),
            "compound_longpress" => Ok(Self::CompoundLongpress),
            "init" => Ok(Self::Init),
            "expand" => Ok(Self::Expand),
            "collapse" => Ok(Self::Collapse),
            "toggle_view" => Ok(Self::ToggleView),
            _ => Err(UnknownActionKindError),
        }
    }
}

impl AsRef<str> for ActionKind {
    /// Returns the string representation of this action kind.
    ///
    /// For `Hold`, returns `"hold"` (the generic prefix).
    /// Use `AtomicAction` for hold_start/hold_stop distinction.
    fn as_ref(&self) -> &str {
        match self {
            Self::Click => "click",
            Self::Longpress => "longpress",
            Self::Hold => "hold",
            Self::DoublePress => "double_press",
            Self::SwipeUp => "swipe_up",
            Self::SwipeDown => "swipe_down",
            Self::RightClick => "right_click",
            Self::MiddleClick => "middle_click",
            Self::ScrollUp => "scroll_up",
            Self::ScrollDown => "scroll_down",
            Self::CompoundLongpress => "compound_longpress",
            Self::Init => "init",
            Self::Expand => "expand",
            Self::Collapse => "collapse",
            Self::ToggleView => "toggle_view",
        }
    }
}

/// Defines how a configured binding interacts with the widget's default behavior.
///
/// Set via TOML fields like `click_mode = "supplement"`.
/// Defaults to `Replace` when omitted.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BindingMode {
    /// The binding replaces the widget's default action (default behavior).
    #[default]
    Replace,
    /// The binding is dispatched in addition to the widget's default action.
    Supplement,
}

/// The resolved routing information for a single widget action.
#[derive(Debug, Clone)]
pub struct ResolvedAction {
    /// The message topic to broadcast to.
    pub topic: String,
    /// The payload to send with the broadcast.
    pub payload: Value,
    /// Optional target instance for the broadcast.
    pub instance: Option<String>,
}

/// Trait for action bindings that can be checked for configuration and dispatched.
///
/// Implemented by `ActionBinding` and all wrapper structs (`ClickBinding`,
/// `LongpressBinding`, etc.) via the `impl_as_binding!` macro.
/// Enables `&dyn DispatchableBinding` dispatch in `dispatch_by_kind` / `dispatch_action`
/// without cloning fields.
pub trait DispatchableBinding {
    /// Returns `true` if this binding has both a topic and a payload configured.
    fn is_configured(&self) -> bool;

    /// Returns `true` if this binding is in supplement mode.
    fn is_supplement(&self) -> bool;

    /// Dispatches this binding's action via the given broadcaster.
    ///
    /// If the binding is not configured, this is a no-op.
    /// If `instance` is set, broadcasts to that specific instance.
    /// Otherwise, broadcasts to all instances.
    fn dispatch(&self, broadcaster: &MessageBroadcasterInner);
}

/// Trait for widgets that have default fallback behavior for action kinds.
///
/// Implemented by widgets that need to execute default behavior when an action
/// binding is not configured, or when a binding is in `BindingMode::Supplement`.
/// The fallback is dispatched in addition to (supplement) or instead of (replace)
/// the configured binding.
pub trait DefaultFallback {
    /// Executes the widget's default fallback action for the given action kind.
    fn default_fallback(&self, kind: &ActionKind, broadcaster: &MessageBroadcasterInner);

    /// Executes the widget's default fallback action for the given action kind,
    /// with the GDK button number that triggered the gesture.
    ///
    /// Defaults to calling `default_fallback` (ignoring the button).
    /// Override this to implement button-specific longpress behavior.
    fn default_fallback_with_button(&self, kind: &ActionKind, button: u32, broadcaster: &MessageBroadcasterInner) {
        let _ = button;
        self.default_fallback(kind, broadcaster);
    }

    /// Executes the widget's default fallback action for a drag gesture,
    /// with the vertical drag offset for distance-proportional effects.
    ///
    /// Defaults to calling `default_fallback` (ignoring the offset).
    /// Override this to implement distance-proportional drag behavior.
    fn default_fallback_drag(&self, kind: &ActionKind, offset_y: f64, broadcaster: &MessageBroadcasterInner) {
        let _ = offset_y;
        self.default_fallback(kind, broadcaster);
    }
}

/// A configurable action binding for a widget input trigger.
///
/// Encapsulates the message topic, payload, target instance, and
/// human-readable description for a single action kind. Used in widget
/// config structs via `#[serde(flatten)]` with per-kind wrapper structs
/// generated by the `action_binding!` macro.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
    /// Dispatch mode: `replace` (default) or `supplement`.
    pub mode: BindingMode,
}

impl ActionBinding {
    /// Returns `true` if this binding has both a topic and a payload configured.
    pub fn is_configured(&self) -> bool {
        self.topic.is_some() && self.payload.is_some()
    }

    /// Resolves this binding into a `ResolvedAction` for broadcasting.
    ///
    /// Returns `None` if the binding is not configured (missing topic or payload).
    pub fn resolve(&self) -> Option<ResolvedAction> {
        let topic = self.topic.clone()?;
        let payload = self.payload.clone()?;
        let instance = self.instance.clone();
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
                broadcaster.broadcast_string_to_instance(&instance, &resolved.topic, &resolved.payload.to_string());
            } else {
                broadcaster.broadcast_string(&resolved.topic, &resolved.payload.to_string());
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

    /// Returns `true` if this binding is in supplement mode.
    pub fn is_supplement(&self) -> bool {
        self.mode == BindingMode::Supplement
    }

    /// Dispatches this binding, executing the fallback if the binding is not configured
    /// or if the binding is in supplement mode.
    ///
    /// - If configured and `replace` mode: dispatches the binding only.
    /// - If configured and `supplement` mode: dispatches the binding, then executes the fallback.
    /// - If not configured: executes the fallback only.
    pub fn dispatch_with_fallback<F: FnOnce()>(&self, broadcaster: &MessageBroadcasterInner, fallback: F) {
        if self.is_configured() {
            self.dispatch(broadcaster);
            if self.is_supplement() {
                fallback();
            }
        } else {
            fallback();
        }
    }
}

impl DispatchableBinding for ActionBinding {
    fn is_configured(&self) -> bool {
        ActionBinding::is_configured(self)
    }

    fn is_supplement(&self) -> bool {
        ActionBinding::is_supplement(self)
    }

    fn dispatch(&self, broadcaster: &MessageBroadcasterInner) {
        ActionBinding::dispatch(self, broadcaster);
    }
}

/// Wrapper struct for the **click** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `click_topic`, `click_payload`, `click_instance`, `click_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ClickBinding {
    /// Message topic for single-click action.
    #[serde(rename = "click_topic")]
    pub topic: Option<String>,
    /// Message payload for single-click action (JSON/TOML).
    #[serde(rename = "click_payload")]
    pub payload: Option<Value>,
    /// Target instance for single-click message.
    #[serde(rename = "click_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the click action does.
    #[serde(rename = "click_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "click_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **longpress** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `longpress_topic`, `longpress_payload`, `longpress_instance`, `longpress_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct LongpressBinding {
    /// Message topic for long-press.
    #[serde(rename = "longpress_topic")]
    pub topic: Option<String>,
    /// Message payload for long-press (JSON/TOML).
    #[serde(rename = "longpress_payload")]
    pub payload: Option<Value>,
    /// Target instance for long-press message.
    #[serde(rename = "longpress_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the longpress action does.
    #[serde(rename = "longpress_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "longpress_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **hold** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `hold_topic`, `hold_payload`, `hold_instance`, `hold_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct HoldBinding {
    /// Message topic for hold (push-to-talk).
    #[serde(rename = "hold_topic")]
    pub topic: Option<String>,
    /// Message payload for hold (JSON/TOML).
    #[serde(rename = "hold_payload")]
    pub payload: Option<Value>,
    /// Target instance for hold message.
    #[serde(rename = "hold_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the hold action does.
    #[serde(rename = "hold_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "hold_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **double press** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `double_press_topic`, `double_press_payload`, `double_press_instance`, `double_press_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DoublePressBinding {
    /// Message topic for double press.
    #[serde(rename = "double_press_topic")]
    pub topic: Option<String>,
    /// Message payload for double press (JSON/TOML).
    #[serde(rename = "double_press_payload")]
    pub payload: Option<Value>,
    /// Target instance for double press message.
    #[serde(rename = "double_press_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the double press action does.
    #[serde(rename = "double_press_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "double_press_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **swipe-up** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `swipe_up_topic`, `swipe_up_payload`, `swipe_up_instance`, `swipe_up_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SwipeUpBinding {
    /// Message topic for swipe-up gesture.
    #[serde(rename = "swipe_up_topic")]
    pub topic: Option<String>,
    /// Message payload for swipe-up gesture (JSON/TOML).
    #[serde(rename = "swipe_up_payload")]
    pub payload: Option<Value>,
    /// Target instance for swipe-up message.
    #[serde(rename = "swipe_up_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the swipe-up action does.
    #[serde(rename = "swipe_up_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "swipe_up_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **swipe-down** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `swipe_down_topic`, `swipe_down_payload`, `swipe_down_instance`, `swipe_down_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SwipeDownBinding {
    /// Message topic for swipe-down gesture.
    #[serde(rename = "swipe_down_topic")]
    pub topic: Option<String>,
    /// Message payload for swipe-down gesture (JSON/TOML).
    #[serde(rename = "swipe_down_payload")]
    pub payload: Option<Value>,
    /// Target instance for swipe-down message.
    #[serde(rename = "swipe_down_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the swipe-down action does.
    #[serde(rename = "swipe_down_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "swipe_down_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **right-click** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `right_click_topic`, `right_click_payload`, `right_click_instance`, `right_click_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct RightClickBinding {
    /// Message topic for right-click (mouse secondary button).
    #[serde(rename = "right_click_topic")]
    pub topic: Option<String>,
    /// Message payload for right-click (JSON/TOML).
    #[serde(rename = "right_click_payload")]
    pub payload: Option<Value>,
    /// Target instance for right-click message.
    #[serde(rename = "right_click_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the right-click action does.
    #[serde(rename = "right_click_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "right_click_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **middle-click** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `middle_click_topic`, `middle_click_payload`, `middle_click_instance`, `middle_click_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct MiddleClickBinding {
    /// Message topic for middle-click (mouse middle button).
    #[serde(rename = "middle_click_topic")]
    pub topic: Option<String>,
    /// Message payload for middle-click (JSON/TOML).
    #[serde(rename = "middle_click_payload")]
    pub payload: Option<Value>,
    /// Target instance for middle-click message.
    #[serde(rename = "middle_click_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the middle-click action does.
    #[serde(rename = "middle_click_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "middle_click_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **scroll-up** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `scroll_up_topic`, `scroll_up_payload`, `scroll_up_instance`, `scroll_up_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ScrollUpBinding {
    /// Message topic for scroll-up (mouse wheel up).
    #[serde(rename = "scroll_up_topic")]
    pub topic: Option<String>,
    /// Message payload for scroll-up (JSON/TOML).
    #[serde(rename = "scroll_up_payload")]
    pub payload: Option<Value>,
    /// Target instance for scroll-up message.
    #[serde(rename = "scroll_up_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the scroll-up action does.
    #[serde(rename = "scroll_up_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "scroll_up_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **scroll-down** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `scroll_down_topic`, `scroll_down_payload`, `scroll_down_instance`, `scroll_down_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ScrollDownBinding {
    /// Message topic for scroll-down (mouse wheel down).
    #[serde(rename = "scroll_down_topic")]
    pub topic: Option<String>,
    /// Message payload for scroll-down (JSON/TOML).
    #[serde(rename = "scroll_down_payload")]
    pub payload: Option<Value>,
    /// Target instance for scroll-down message.
    #[serde(rename = "scroll_down_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the scroll-down action does.
    #[serde(rename = "scroll_down_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "scroll_down_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **compound longpress** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `compound_longpress_topic`, `compound_longpress_payload`, `compound_longpress_instance`, `compound_longpress_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CompoundLongpressBinding {
    /// Message topic for compound longpress.
    #[serde(rename = "compound_longpress_topic")]
    pub topic: Option<String>,
    /// Message payload for compound longpress (JSON/TOML).
    #[serde(rename = "compound_longpress_payload")]
    pub payload: Option<Value>,
    /// Target instance for compound longpress message.
    #[serde(rename = "compound_longpress_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the compound longpress action does.
    #[serde(rename = "compound_longpress_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "compound_longpress_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **init** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `init_topic`, `init_payload`, `init_instance`, `init_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct InitBinding {
    /// Initial one-shot request topic (sent on widget construction).
    #[serde(rename = "init_topic")]
    pub topic: Option<String>,
    /// Initial one-shot request payload (JSON/TOML).
    #[serde(rename = "init_payload")]
    pub payload: Option<Value>,
    /// Target instance for the initial one-shot request.
    #[serde(rename = "init_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the init action does.
    #[serde(rename = "init_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "init_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **expand** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `expand_topic`, `expand_payload`, `expand_instance`, `expand_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ExpandBinding {
    /// Message topic for expand action.
    #[serde(rename = "expand_topic")]
    pub topic: Option<String>,
    /// Message payload for expand action (JSON/TOML).
    #[serde(rename = "expand_payload")]
    pub payload: Option<Value>,
    /// Target instance for expand message.
    #[serde(rename = "expand_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the expand action does.
    #[serde(rename = "expand_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "expand_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **collapse** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `collapse_topic`, `collapse_payload`, `collapse_instance`, `collapse_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CollapseBinding {
    /// Message topic for collapse action.
    #[serde(rename = "collapse_topic")]
    pub topic: Option<String>,
    /// Message payload for collapse action (JSON/TOML).
    #[serde(rename = "collapse_payload")]
    pub payload: Option<Value>,
    /// Target instance for collapse message.
    #[serde(rename = "collapse_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the collapse action does.
    #[serde(rename = "collapse_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "collapse_mode")]
    pub mode: BindingMode,
}

/// Wrapper struct for the **toggle_view** action binding.
///
/// Flattened into widget config structs via `#[serde(flatten)]`.
/// TOML field names: `toggle_view_topic`, `toggle_view_payload`, `toggle_view_instance`, `toggle_view_description`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ToggleViewBinding {
    /// Message topic for toggle_view action.
    #[serde(rename = "toggle_view_topic")]
    pub topic: Option<String>,
    /// Message payload for toggle_view action (JSON/TOML).
    #[serde(rename = "toggle_view_payload")]
    pub payload: Option<Value>,
    /// Target instance for toggle_view message.
    #[serde(rename = "toggle_view_instance")]
    pub instance: Option<String>,
    /// Human-readable description of what the toggle_view action does.
    #[serde(rename = "toggle_view_description")]
    pub description: Option<String>,
    /// Dispatch mode: `replace` (default) or `supplement`.
    #[serde(rename = "toggle_view_mode")]
    pub mode: BindingMode,
}

/// Generates the `as_binding()` method and `DispatchableBinding` impl for a wrapper struct.
///
/// Since all wrapper structs share the same four fields (`topic`, `payload`,
/// `instance`, `description`), this macro provides a concise way to implement
/// the conversion to `ActionBinding` and the `DispatchableBinding` trait
/// without repeating the method bodies.
#[macro_export]
macro_rules! impl_as_binding {
    ($name:ident) => {
        impl $name {
            /// Returns an owned `ActionBinding` with this binding's fields.
            ///
            /// This provides access to the reusable `resolve()`,
            /// `as_tool_description()` methods, and owned capture for GTK closures.
            pub fn as_binding(&self) -> $crate::action::ActionBinding {
                $crate::action::ActionBinding {
                    topic: self.topic.clone(),
                    payload: self.payload.clone(),
                    instance: self.instance.clone(),
                    description: self.description.clone(),
                    mode: self.mode,
                }
            }

            /// Returns `true` if this binding has both a topic and a payload configured.
            pub fn is_configured(&self) -> bool {
                self.topic.is_some() && self.payload.is_some()
            }

            /// Returns `true` if this binding is in supplement mode.
            pub fn is_supplement(&self) -> bool {
                self.mode == $crate::action::BindingMode::Supplement
            }

            /// Dispatches this binding via the given broadcaster, respecting `instance`.
            pub fn dispatch(&self, broadcaster: &$crate::MessageBroadcasterInner) {
                self.as_binding().dispatch(broadcaster);
            }

            /// Dispatches this binding, executing the fallback if the binding is not configured
            /// or if the binding is in supplement mode.
            ///
            /// - If configured and `replace` mode: dispatches the binding only.
            /// - If configured and `supplement` mode: dispatches the binding, then executes the fallback.
            /// - If not configured: executes the fallback only.
            pub fn dispatch_with_fallback<F: FnOnce()>(&self, broadcaster: &$crate::MessageBroadcasterInner, fallback: F) {
                if self.is_configured() {
                    self.dispatch(broadcaster);
                    if self.is_supplement() {
                        fallback();
                    }
                } else {
                    fallback();
                }
            }
        }

        impl $crate::action::DispatchableBinding for $name {
            fn is_configured(&self) -> bool {
                self.topic.is_some() && self.payload.is_some()
            }

            fn is_supplement(&self) -> bool {
                self.mode == $crate::action::BindingMode::Supplement
            }

            fn dispatch(&self, broadcaster: &$crate::MessageBroadcasterInner) {
                self.as_binding().dispatch(broadcaster);
            }
        }
    };
}

impl_as_binding!(ClickBinding);
impl_as_binding!(LongpressBinding);
impl_as_binding!(HoldBinding);
impl_as_binding!(DoublePressBinding);
impl_as_binding!(SwipeUpBinding);
impl_as_binding!(SwipeDownBinding);
impl_as_binding!(RightClickBinding);
impl_as_binding!(MiddleClickBinding);
impl_as_binding!(ScrollUpBinding);
impl_as_binding!(ScrollDownBinding);
impl_as_binding!(CompoundLongpressBinding);
impl_as_binding!(InitBinding);
impl_as_binding!(ExpandBinding);
impl_as_binding!(CollapseBinding);
impl_as_binding!(ToggleViewBinding);

/// A reusable collection of all action binding types for a widget.
///
/// Designed to be embedded in widget config structs via `#[serde(flatten)]`.
/// Each inner binding is also `#[serde(flatten)]`, so all TOML/JSON keys
/// (e.g. `click_topic`, `longpress_payload`, ...) are lifted directly into
/// the parent config's namespace — identical to declaring each binding field
/// individually.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ActionBindings {
    /// Single-click action binding.
    #[serde(flatten)]
    pub click: ClickBinding,
    /// Long-press action binding.
    #[serde(flatten)]
    pub longpress: LongpressBinding,
    /// Hold action binding (push-to-talk).
    #[serde(flatten)]
    pub hold: HoldBinding,
    /// Double-press action binding.
    #[serde(flatten)]
    pub double_press: DoublePressBinding,
    /// Swipe-up gesture action binding.
    #[serde(flatten)]
    pub swipe_up: SwipeUpBinding,
    /// Swipe-down gesture action binding.
    #[serde(flatten)]
    pub swipe_down: SwipeDownBinding,
    /// Right-click action binding (mouse secondary button).
    #[serde(flatten)]
    pub right_click: RightClickBinding,
    /// Middle-click action binding (mouse middle button).
    #[serde(flatten)]
    pub middle_click: MiddleClickBinding,
    /// Scroll-up action binding (mouse wheel up).
    #[serde(flatten)]
    pub scroll_up: ScrollUpBinding,
    /// Scroll-down action binding (mouse wheel down).
    #[serde(flatten)]
    pub scroll_down: ScrollDownBinding,
    /// Compound longpress action binding.
    #[serde(flatten)]
    pub compound_longpress: CompoundLongpressBinding,
    /// Initial one-shot request binding (sent on widget construction).
    #[serde(flatten)]
    pub init: InitBinding,
    /// Expand action binding (MCP tool action).
    #[serde(flatten)]
    pub expand: ExpandBinding,
    /// Collapse action binding (MCP tool action).
    #[serde(flatten)]
    pub collapse: CollapseBinding,
    /// Toggle view action binding (MCP tool action).
    #[serde(flatten)]
    pub toggle_view: ToggleViewBinding,
}

impl ActionBindings {
    /// Returns the binding for the given action kind as a `&dyn DispatchableBinding`.
    pub fn binding_for_kind(&self, kind: ActionKind) -> &dyn DispatchableBinding {
        match kind {
            ActionKind::Click => &self.click,
            ActionKind::Longpress => &self.longpress,
            ActionKind::Hold => &self.hold,
            ActionKind::DoublePress => &self.double_press,
            ActionKind::SwipeUp => &self.swipe_up,
            ActionKind::SwipeDown => &self.swipe_down,
            ActionKind::RightClick => &self.right_click,
            ActionKind::MiddleClick => &self.middle_click,
            ActionKind::ScrollUp => &self.scroll_up,
            ActionKind::ScrollDown => &self.scroll_down,
            ActionKind::CompoundLongpress => &self.compound_longpress,
            ActionKind::Init => &self.init,
            ActionKind::Expand => &self.expand,
            ActionKind::Collapse => &self.collapse,
            ActionKind::ToggleView => &self.toggle_view,
        }
    }

    /// Dispatches an action kind via the broadcaster, respecting `instance`.
    ///
    /// Returns `true` if the action was configured and dispatched, `false` otherwise.
    pub fn dispatch_by_kind(&self, kind: ActionKind, broadcaster: &MessageBroadcasterInner) -> bool {
        let binding = self.binding_for_kind(kind);
        if binding.is_configured() {
            binding.dispatch(broadcaster);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_binding_is_configured() {
        let empty = ActionBinding::default();
        assert!(!empty.is_configured());

        let with_topic_only = ActionBinding {
            topic: Some("test.topic".to_string()),
            ..Default::default()
        };
        assert!(!with_topic_only.is_configured());

        let configured = ActionBinding {
            topic: Some("test.topic".to_string()),
            payload: Some(Value::String("test".to_string())),
            ..Default::default()
        };
        assert!(configured.is_configured());
    }

    #[test]
    fn test_action_binding_resolve() {
        let empty = ActionBinding::default();
        assert!(empty.resolve().is_none());

        let configured = ActionBinding {
            topic: Some("test.topic".to_string()),
            payload: Some(Value::String("payload".to_string())),
            instance: Some("instance1".to_string()),
            description: None,
            mode: BindingMode::default(),
        };
        let resolved = configured.resolve().unwrap();
        assert_eq!(resolved.topic, "test.topic");
        assert_eq!(resolved.payload, Value::String("payload".to_string()));
        assert_eq!(resolved.instance, Some("instance1".to_string()));
    }

    #[test]
    fn test_action_binding_as_tool_description() {
        let empty = ActionBinding::default();
        assert!(empty.as_tool_description().is_none());

        let with_desc = ActionBinding {
            description: Some("Turns the light on".to_string()),
            ..Default::default()
        };
        assert_eq!(with_desc.as_tool_description(), Some("Turns the light on"));
    }

    #[test]
    fn test_action_kind_from_str() {
        assert_eq!(ActionKind::from_str("click"), Ok(ActionKind::Click));
        assert_eq!(ActionKind::from_str("longpress"), Ok(ActionKind::Longpress));
        assert_eq!(ActionKind::from_str("hold_start"), Ok(ActionKind::Hold));
        assert_eq!(ActionKind::from_str("hold_stop"), Ok(ActionKind::Hold));
        assert_eq!(ActionKind::from_str("double_press"), Ok(ActionKind::DoublePress));
        assert_eq!(ActionKind::from_str("swipe_up"), Ok(ActionKind::SwipeUp));
        assert_eq!(ActionKind::from_str("swipe_down"), Ok(ActionKind::SwipeDown));
        assert_eq!(ActionKind::from_str("right_click"), Ok(ActionKind::RightClick));
        assert_eq!(ActionKind::from_str("middle_click"), Ok(ActionKind::MiddleClick));
        assert_eq!(ActionKind::from_str("scroll_up"), Ok(ActionKind::ScrollUp));
        assert_eq!(ActionKind::from_str("scroll_down"), Ok(ActionKind::ScrollDown));
        assert_eq!(ActionKind::from_str("compound_longpress"), Ok(ActionKind::CompoundLongpress));
        assert_eq!(ActionKind::from_str("init"), Ok(ActionKind::Init));
        assert_eq!(ActionKind::from_str("expand"), Ok(ActionKind::Expand));
        assert_eq!(ActionKind::from_str("collapse"), Ok(ActionKind::Collapse));
        assert_eq!(ActionKind::from_str("toggle_view"), Ok(ActionKind::ToggleView));
        assert!(ActionKind::from_str("unknown").is_err());
    }

    #[test]
    fn test_action_kind_as_ref() {
        assert_eq!(ActionKind::Click.as_ref(), "click");
        assert_eq!(ActionKind::Longpress.as_ref(), "longpress");
        assert_eq!(ActionKind::Hold.as_ref(), "hold");
        assert_eq!(ActionKind::DoublePress.as_ref(), "double_press");
        assert_eq!(ActionKind::SwipeUp.as_ref(), "swipe_up");
        assert_eq!(ActionKind::SwipeDown.as_ref(), "swipe_down");
        assert_eq!(ActionKind::RightClick.as_ref(), "right_click");
        assert_eq!(ActionKind::MiddleClick.as_ref(), "middle_click");
        assert_eq!(ActionKind::ScrollUp.as_ref(), "scroll_up");
        assert_eq!(ActionKind::ScrollDown.as_ref(), "scroll_down");
        assert_eq!(ActionKind::CompoundLongpress.as_ref(), "compound_longpress");
        assert_eq!(ActionKind::Init.as_ref(), "init");
        assert_eq!(ActionKind::Expand.as_ref(), "expand");
        assert_eq!(ActionKind::Collapse.as_ref(), "collapse");
        assert_eq!(ActionKind::ToggleView.as_ref(), "toggle_view");
    }

    #[test]
    fn test_click_binding_serde_flatten() {
        #[derive(Debug, Default, Deserialize)]
        #[serde(default)]
        struct TestConfig {
            #[serde(flatten)]
            click: ClickBinding,
            #[serde(flatten)]
            longpress: LongpressBinding,
        }

        let toml_str = r#"
click_topic = "service.test.click"
click_payload = { action = "on" }
click_description = "Turns on"
longpress_topic = "service.test.longpress"
longpress_payload = { action = "off" }
longpress_instance = "instance1"
"#;
        let config: TestConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.click.topic.as_deref(), Some("service.test.click"));
        assert!(config.click.payload.is_some());
        assert_eq!(config.click.description.as_deref(), Some("Turns on"));
        assert!(config.click.instance.is_none());

        assert_eq!(config.longpress.topic.as_deref(), Some("service.test.longpress"));
        assert!(config.longpress.payload.is_some());
        assert_eq!(config.longpress.instance.as_deref(), Some("instance1"));
        assert!(config.longpress.description.is_none());
    }

    #[test]
    fn test_click_binding_default() {
        let binding = ClickBinding::default();
        assert!(binding.topic.is_none());
        assert!(binding.payload.is_none());
        assert!(binding.instance.is_none());
        assert!(binding.description.is_none());
    }

    #[test]
    fn test_click_binding_as_binding() {
        let binding = ClickBinding {
            topic: Some("topic".to_string()),
            payload: Some(Value::Null),
            instance: Some("inst".to_string()),
            description: Some("desc".to_string()),
            mode: BindingMode::default(),
        };
        let action = binding.as_binding();
        assert_eq!(action.topic.as_deref(), Some("topic"));
        assert_eq!(action.payload, Some(Value::Null));
        assert_eq!(action.instance.as_deref(), Some("inst"));
        assert_eq!(action.description.as_deref(), Some("desc"));
        assert!(action.is_configured());
    }

    #[test]
    fn test_binding_mode_default_is_replace() {
        assert_eq!(BindingMode::default(), BindingMode::Replace);
    }

    #[test]
    fn test_action_binding_is_supplement() {
        let replace_binding = ActionBinding {
            topic: Some("topic".to_string()),
            payload: Some(Value::Null),
            instance: None,
            description: None,
            mode: BindingMode::Replace,
        };
        assert!(!replace_binding.is_supplement());

        let supplement_binding = ActionBinding {
            topic: Some("topic".to_string()),
            payload: Some(Value::Null),
            instance: None,
            description: None,
            mode: BindingMode::Supplement,
        };
        assert!(supplement_binding.is_supplement());
    }

    #[test]
    fn test_click_binding_is_supplement() {
        let replace = ClickBinding {
            topic: Some("topic".to_string()),
            payload: Some(Value::Null),
            instance: None,
            description: None,
            mode: BindingMode::Replace,
        };
        assert!(!replace.is_supplement());

        let supplement = ClickBinding {
            topic: Some("topic".to_string()),
            payload: Some(Value::Null),
            instance: None,
            description: None,
            mode: BindingMode::Supplement,
        };
        assert!(supplement.is_supplement());
    }

    #[test]
    fn test_binding_mode_serde() {
        #[derive(Debug, Default, Deserialize)]
        #[serde(default)]
        struct TestConfig {
            #[serde(flatten)]
            click: ClickBinding,
        }

        let toml_str = r#"
click_topic = "service.test.click"
click_payload = { action = "on" }
click_mode = "supplement"
"#;
        let config: TestConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.click.mode, BindingMode::Supplement);

        let toml_str_no_mode = r#"
click_topic = "service.test.click"
click_payload = { action = "on" }
"#;
        let config_no_mode: TestConfig = toml::from_str(toml_str_no_mode).unwrap();
        assert_eq!(config_no_mode.click.mode, BindingMode::Replace);
    }
}
