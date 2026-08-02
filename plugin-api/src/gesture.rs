//! Shared GTK gesture handler setup for widgets using `ActionBindings`.
//!
//! Provides the `GestureHandler` trait with a blanket implementation for all
//! `DefaultFallback + 'static` types. The `attach_gesture_handlers` method
//! eliminates boilerplate when wiring up click, longpress, drag (swipe), and
//! scroll gesture controllers on a widget. Each gesture uses
//! `dispatch_with_fallback` so that configured bindings are dispatched, and the
//! widget's own `DefaultFallback` is used when no binding is configured or when
//! a binding is in `BindingMode::Supplement`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;

use glib::Propagation;
use gtk4::EventControllerScroll;
use gtk4::EventControllerScrollFlags;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::GestureDrag;
use gtk4::GestureLongPress;
use gtk4::PropagationPhase;
use gtk4::gdk;
use gtk4::prelude::EventControllerExt;
use gtk4::prelude::GestureDragExt;
use gtk4::prelude::GestureExt;
use gtk4::prelude::GestureSingleExt;
use gtk4::prelude::ObjectExt;
use gtk4::prelude::WidgetExt;

use crate::action::ActionBindings;
use crate::action::ActionKind;
use crate::action::DefaultFallback;
use crate::messages::MessageBroadcasterInner;

/// Default swipe threshold in pixels for drag gestures.
pub const DEFAULT_SWIPE_THRESHOLD: f64 = 50.0;

/// Configuration for gesture handler setup.
///
/// Controls swipe threshold, longpress delay factor, gesture grouping,
/// longpress visual feedback, and scroll throttling.
/// `Default` provides standard values.
#[derive(Debug, Clone)]
pub struct GestureHandlersConfiguration {
    /// Minimum vertical drag distance in pixels to trigger swipe actions.
    pub swipe_threshold: f64,
    /// Optional delay factor for `GestureLongPress`. When `None`, the GTK default is used.
    pub delay_factor: Option<f64>,
    /// When `true`, groups `GestureLongPress` with `GestureClick` via `group_with`.
    /// Prevents click and longpress from firing simultaneously on the same event sequence.
    pub group_gestures: bool,
    /// Optional CSS class added to the widget on longpress begin and removed on end/cancel.
    pub longpress_css_class: Option<String>,
    /// Optional drag throttle interval in milliseconds.
    /// When `Some(ms)`, drag-end events within this interval after the last dispatch are ignored.
    /// When `None`, no throttling is applied.
    pub drag_throttling: Option<u64>,
    /// When `true`, attaches a `GestureDrag` controller for swipe up/down gestures.
    /// Set to `false` when the widget manages its own drag logic.
    pub drag_enabled: bool,
    /// Optional scroll throttle interval in milliseconds.
    /// When `Some(ms)`, scroll events within this interval after the last dispatch are ignored.
    /// When `None`, no throttling is applied.
    pub scroll_throttling: Option<u64>,
}

impl Default for GestureHandlersConfiguration {
    fn default() -> Self {
        Self {
            swipe_threshold: DEFAULT_SWIPE_THRESHOLD,
            delay_factor: None,
            group_gestures: true,
            longpress_css_class: None,
            drag_throttling: None,
            drag_enabled: true,
            scroll_throttling: None,
        }
    }
}

/// Trait that provides GTK gesture handler setup for widgets implementing `DefaultFallback`.
///
/// A blanket implementation is provided for all `T: DefaultFallback + 'static`,
/// so widgets only need to implement `DefaultFallback` to gain access to
/// `attach_gesture_handlers`.
pub trait GestureHandler: DefaultFallback + 'static {
    /// Attaches click, longpress, drag (swipe), and scroll gesture handlers to the given widget.
    ///
    /// All gestures use `dispatch_with_fallback` so that configured bindings are dispatched.
    /// When a binding is not configured (or is in `Supplement` mode), the widget's own
    /// `DefaultFallback::default_fallback` is called with the corresponding `ActionKind`.
    ///
    /// # Parameters
    ///
    /// - `widget`: The GTK widget to attach gesture controllers to.
    /// - `actions`: The action bindings configuration.
    /// - `broadcaster`: The message broadcaster for dispatching configured bindings.
    /// - `config`: Gesture handler configuration (swipe threshold, delay factor, etc.).
    fn attach_gesture_handlers(
        self: &Rc<Self>,
        widget: &gtk4::Widget,
        actions: &ActionBindings,
        broadcaster: &MessageBroadcasterInner,
        config: &GestureHandlersConfiguration,
    );
}

impl<T: DefaultFallback + 'static> GestureHandler for T {
    fn attach_gesture_handlers(
        self: &Rc<Self>,
        widget: &gtk4::Widget,
        actions: &ActionBindings,
        broadcaster: &MessageBroadcasterInner,
        config: &GestureHandlersConfiguration,
    ) {
        let click_binding = actions.click.as_binding();
        let double_press_binding = actions.double_press.as_binding();
        let right_click_binding = actions.right_click.as_binding();
        let middle_click_binding = actions.middle_click.as_binding();
        let longpress_binding = actions.longpress.as_binding();
        let swipe_up_binding = actions.swipe_up.as_binding();
        let swipe_down_binding = actions.swipe_down.as_binding();
        let scroll_up_binding = actions.scroll_up.as_binding();
        let scroll_down_binding = actions.scroll_down.as_binding();

        let click_gesture = GestureClick::builder().button(0).propagation_phase(PropagationPhase::Capture).build();
        let click_broadcaster = broadcaster.clone();
        let click_fallback = self.clone();
        click_gesture.connect_released(move |gesture, n_clicks, _, _| {
            if let Some(seq) = gesture.current_sequence() {
                let state = gesture.sequence_state(&seq);
                if state == EventSequenceState::Claimed || state == EventSequenceState::Denied {
                    return;
                }
            }
            let button = gesture.current_button();
            match button {
                gdk::BUTTON_PRIMARY => {
                    if n_clicks == 2 {
                        double_press_binding.dispatch_with_fallback(&click_broadcaster, || {
                            click_fallback.default_fallback(&ActionKind::DoublePress, &click_broadcaster);
                        });
                    } else {
                        click_binding.dispatch_with_fallback(&click_broadcaster, || {
                            click_fallback.default_fallback(&ActionKind::Click, &click_broadcaster);
                        });
                    }
                }
                gdk::BUTTON_SECONDARY => {
                    right_click_binding.dispatch_with_fallback(&click_broadcaster, || {
                        click_fallback.default_fallback(&ActionKind::RightClick, &click_broadcaster);
                    });
                }
                gdk::BUTTON_MIDDLE => {
                    middle_click_binding.dispatch_with_fallback(&click_broadcaster, || {
                        click_fallback.default_fallback(&ActionKind::MiddleClick, &click_broadcaster);
                    });
                }
                _ => {}
            }
            gesture.set_state(EventSequenceState::Claimed);
        });
        widget.add_controller(click_gesture.clone());

        let mut longpress_builder = GestureLongPress::builder().button(0).propagation_phase(PropagationPhase::Capture);
        if let Some(delay_factor) = config.delay_factor {
            longpress_builder = longpress_builder.delay_factor(delay_factor);
        }
        let longpress_gesture = longpress_builder.build();
        if config.group_gestures {
            longpress_gesture.group_with(&click_gesture);
        }
        let longpress_broadcaster = broadcaster.clone();
        let longpress_fallback = self.clone();
        longpress_gesture.connect_pressed(move |gesture, _x, _y| {
            let button = gesture.current_button();
            longpress_binding.dispatch_with_fallback(&longpress_broadcaster, || {
                longpress_fallback.default_fallback_with_button(&ActionKind::Longpress, button, &longpress_broadcaster);
            });
            gesture.set_state(EventSequenceState::Claimed);
        });
        if let Some(ref css_class) = config.longpress_css_class {
            let css_class_begin = css_class.clone();
            let widget_weak = widget.downgrade();
            longpress_gesture.connect_begin(move |_, _| {
                if let Some(w) = widget_weak.upgrade() {
                    w.add_css_class(&css_class_begin);
                }
            });
            let css_class_end = css_class.clone();
            let widget_weak = widget.downgrade();
            longpress_gesture.connect_end(move |_, _| {
                if let Some(w) = widget_weak.upgrade() {
                    w.remove_css_class(&css_class_end);
                }
            });
            let css_class_cancel = css_class.clone();
            let widget_weak = widget.downgrade();
            longpress_gesture.connect_cancelled(move |gesture| {
                if let Some(w) = widget_weak.upgrade() {
                    w.remove_css_class(&css_class_cancel);
                }
                gesture.set_state(EventSequenceState::None);
            });
        }
        widget.add_controller(longpress_gesture);

        if config.drag_enabled {
            let drag_gesture = GestureDrag::new();
            drag_gesture.set_propagation_phase(PropagationPhase::Capture);
            let drag_broadcaster = broadcaster.clone();
            let drag_fallback = self.clone();
            let swipe_threshold = config.swipe_threshold;
            let drag_throttle_ms = config.drag_throttling;
            let drag_throttle = drag_throttle_ms.map(|ms| Rc::new(RefCell::new(Instant::now() - Duration::from_millis(ms))));
            drag_gesture.connect_drag_end(move |gesture, offset_x, offset_y| {
                if offset_y.abs() > offset_x.abs() && offset_y.abs() > swipe_threshold {
                    if let (Some(throttle), Some(ms)) = (&drag_throttle, drag_throttle_ms) {
                        let elapsed = {
                            let last = throttle.borrow();
                            Instant::now().duration_since(*last)
                        };
                        if elapsed < Duration::from_millis(ms) {
                            return;
                        }
                    }
                    gesture.set_state(EventSequenceState::Claimed);
                    if offset_y < 0.0 {
                        if let Some(ref throttle) = drag_throttle {
                            *throttle.borrow_mut() = Instant::now();
                        }
                        swipe_up_binding.dispatch_with_fallback(&drag_broadcaster, || {
                            drag_fallback.default_fallback_drag(&ActionKind::SwipeUp, offset_y, &drag_broadcaster);
                        });
                    } else {
                        if let Some(ref throttle) = drag_throttle {
                            *throttle.borrow_mut() = Instant::now();
                        }
                        swipe_down_binding.dispatch_with_fallback(&drag_broadcaster, || {
                            drag_fallback.default_fallback_drag(&ActionKind::SwipeDown, offset_y, &drag_broadcaster);
                        });
                    }
                }
            });
            widget.add_controller(drag_gesture);
        }

        let scroll_controller = EventControllerScroll::builder()
            .flags(EventControllerScrollFlags::VERTICAL)
            .propagation_phase(PropagationPhase::Capture)
            .build();
        let scroll_broadcaster = broadcaster.clone();
        let scroll_fallback = self.clone();
        let scroll_throttle_ms = config.scroll_throttling;
        let scroll_throttle = scroll_throttle_ms.map(|ms| Rc::new(RefCell::new(Instant::now() - Duration::from_millis(ms))));
        scroll_controller.connect_scroll(move |_controller, _dx, dy| {
            if let (Some(throttle), Some(ms)) = (&scroll_throttle, scroll_throttle_ms) {
                let elapsed = {
                    let last = throttle.borrow();
                    Instant::now().duration_since(*last)
                };
                if elapsed < Duration::from_millis(ms) {
                    return Propagation::Stop;
                }
            }
            if dy < 0.0 {
                if let Some(ref throttle) = scroll_throttle {
                    *throttle.borrow_mut() = Instant::now();
                }
                scroll_up_binding.dispatch_with_fallback(&scroll_broadcaster, || {
                    scroll_fallback.default_fallback(&ActionKind::ScrollUp, &scroll_broadcaster);
                });
            } else if dy > 0.0 {
                if let Some(ref throttle) = scroll_throttle {
                    *throttle.borrow_mut() = Instant::now();
                }
                scroll_down_binding.dispatch_with_fallback(&scroll_broadcaster, || {
                    scroll_fallback.default_fallback(&ActionKind::ScrollDown, &scroll_broadcaster);
                });
            }
            Propagation::Stop
        });
        widget.add_controller(scroll_controller);
    }
}
