use gtk4::Widget;
use gtk4::glib;
use gtk4::prelude::*;
use smearor_model_area::AreaTransition;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tracing::error;

/// Manages layout animations for smooth transitions
pub struct LayoutTransition {
    /// Default animation duration in milliseconds
    duration_ms: u32,
}

impl LayoutTransition {
    /// Create a new LayoutTransition with default duration
    pub fn new() -> Self {
        Self { duration_ms: 300 }
    }

    /// Create a new LayoutTransition with custom duration
    pub fn with_duration(duration_ms: u32) -> Self {
        Self { duration_ms }
    }

    /// Animate a width change for smooth layout transitions
    pub fn animate_width_change(&self, widget: &Widget, target_width: i32) {
        let current_width = widget.width_request();
        if current_width == target_width {
            return;
        }

        let widget_clone = widget.clone();
        let duration = self.duration_ms;
        let steps = 10;
        let step_duration = Duration::from_millis(duration as u64 / steps as u64);
        let width_step = (target_width - current_width) as f32 / steps as f32;

        for step in 0..=steps {
            let widget_step = widget_clone.clone();
            let new_width = current_width as f32 + (width_step * step as f32);
            let delay = step_duration * step as u32;

            glib::timeout_add_local(delay, move || {
                widget_step.set_width_request(new_width as i32);
                glib::ControlFlow::Continue
            });
        }
    }

    /// Animate widget addition with smooth transition
    pub fn animate_widget_addition(&self, widget: &Widget, transition: &AreaTransition) {
        error!("transition {transition:?}");
        if transition == &AreaTransition::None {
            return;
        }
        let widget = widget.clone();
        let duration = self.duration_ms;

        let Some(transition_params) = transition.enter_transition_parameters() else {
            return;
        };

        error!("start_opacity {}", transition_params.start_opacity);
        // Set initial state immediately so widget starts invisible
        widget.set_opacity(transition_params.start_opacity);
        if transition_params.start_margin_x != 0 {
            widget.set_margin_start(transition_params.start_margin_x);
        }
        if transition_params.start_margin_y != 0 {
            widget.set_margin_top(transition_params.start_margin_y);
        }

        let start_time = glib::monotonic_time();
        let duration_us = (duration as i64) * 1000;

        let handler_id = Rc::new(RefCell::new(None::<glib::SourceId>));
        let handler_id_clone = handler_id.clone();

        let id = glib::timeout_add_local(Duration::from_millis(16), move || {
            let elapsed = glib::monotonic_time() - start_time;
            let progress = (elapsed as f64 / duration_us as f64).min(1.0);
            let eased = Self::ease_out_cubic(progress);

            let opacity = transition_params.start_opacity + (transition_params.target_opacity - transition_params.start_opacity) * eased;
            let margin_x = transition_params.start_margin_x + ((transition_params.target_margin_x - transition_params.start_margin_x) as f64 * eased) as i32;
            let margin_y = transition_params.start_margin_y + ((transition_params.target_margin_y - transition_params.start_margin_y) as f64 * eased) as i32;

            widget.set_opacity(opacity);
            if transition_params.start_margin_x != 0 || transition_params.target_margin_x != 0 {
                widget.set_margin_start(margin_x);
            }
            if transition_params.start_margin_y != 0 || transition_params.target_margin_y != 0 {
                widget.set_margin_top(margin_y);
            }
            widget.queue_draw();

            if progress >= 1.0 {
                if let Some(id) = handler_id_clone.borrow_mut().take() {
                    id.remove();
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        *handler_id.borrow_mut() = Some(id);
    }

    /// Animate widget removal with smooth transition
    pub fn animate_widget_removal(&self, widget: &Widget, transition: &AreaTransition, callback: impl Fn() + 'static) {
        if transition == &AreaTransition::None {
            callback();
            return;
        }
        let widget = widget.clone();
        let duration = self.duration_ms;

        let Some(transition_params) = transition.exit_transition_parameters() else {
            callback();
            return;
        };

        widget.set_opacity(transition_params.start_opacity);
        if transition_params.start_margin_x != 0 {
            widget.set_margin_start(transition_params.start_margin_x);
        }
        if transition_params.start_margin_y != 0 {
            widget.set_margin_top(transition_params.start_margin_y);
        }

        let start_time = glib::monotonic_time();
        let duration_us = (duration as i64) * 1000;

        let handler_id = Rc::new(RefCell::new(None::<glib::SourceId>));
        let handler_id_clone = handler_id.clone();
        let callback = Rc::new(RefCell::new(Some(callback)));
        let callback_clone = callback.clone();

        let id = glib::timeout_add_local(Duration::from_millis(16), move || {
            let elapsed = glib::monotonic_time() - start_time;
            let progress = (elapsed as f64 / duration_us as f64).min(1.0);
            let eased = Self::ease_out_cubic(progress);

            let opacity = transition_params.start_opacity + (transition_params.target_opacity - transition_params.start_opacity) * eased;
            let margin_x = transition_params.start_margin_x + ((transition_params.target_margin_x - transition_params.start_margin_x) as f64 * eased) as i32;
            let margin_y = transition_params.start_margin_y + ((transition_params.target_margin_y - transition_params.start_margin_y) as f64 * eased) as i32;

            widget.set_opacity(opacity);
            if transition_params.start_margin_x != 0 || transition_params.target_margin_x != 0 {
                widget.set_margin_start(margin_x);
            }
            if transition_params.start_margin_y != 0 || transition_params.target_margin_y != 0 {
                widget.set_margin_top(margin_y);
            }
            widget.queue_draw();

            if progress >= 1.0 {
                if let Some(id) = handler_id_clone.borrow_mut().take() {
                    id.remove();
                }
                if let Some(cb) = callback_clone.borrow_mut().take() {
                    cb();
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        *handler_id.borrow_mut() = Some(id);
    }

    /// Ease-out cubic function for smooth deceleration
    fn ease_out_cubic(t: f64) -> f64 {
        1.0 - (1.0 - t).powi(3)
    }
}

impl Default for LayoutTransition {
    fn default() -> Self {
        Self::new()
    }
}
