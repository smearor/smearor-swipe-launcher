use crate::config::area::transition::AreaTransition;
use gtk4::Widget;
use gtk4::glib;
use gtk4::prelude::*;
use std::time::Duration;

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

    /// Animate widget addition with different transition types
    pub fn animate_widget_addition(&self, widget: &Widget, transition: &AreaTransition) {
        match transition {
            AreaTransition::None => {
                // No animation
            }
            AreaTransition::Fade => {
                widget.set_opacity(0.0);
                let widget_clone = widget.clone();

                glib::timeout_add_local(Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    glib::ControlFlow::Break
                });
            }
            AreaTransition::SlideLeft => {
                widget.set_opacity(0.0);
                let widget_clone = widget.clone();

                glib::timeout_add_local(Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    glib::ControlFlow::Break
                });
            }
            AreaTransition::SlideRight => {
                widget.set_opacity(0.0);
                let widget_clone = widget.clone();

                glib::timeout_add_local(Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    glib::ControlFlow::Break
                });
            }
            AreaTransition::SlideUp => {
                widget.set_opacity(0.0);
                let widget_clone = widget.clone();

                glib::timeout_add_local(Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    glib::ControlFlow::Break
                });
            }
            AreaTransition::SlideDown => {
                widget.set_opacity(0.0);
                let widget_clone = widget.clone();

                glib::timeout_add_local(Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    glib::ControlFlow::Break
                });
            }
            AreaTransition::Pop => {
                widget.set_opacity(0.0);
                let widget_clone = widget.clone();

                glib::timeout_add_local(Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    glib::ControlFlow::Break
                });
            }
            AreaTransition::Scale => {
                widget.set_opacity(0.0);
                let widget_clone = widget.clone();

                glib::timeout_add_local(Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    glib::ControlFlow::Break
                });
            }
        }
    }

    /// Animate widget removal
    pub fn animate_widget_removal(&self, widget: &Widget, transition: &AreaTransition, callback: impl Fn() + 'static) {
        match transition {
            AreaTransition::None => {
                callback();
            }
            AreaTransition::Fade => {
                let widget_clone = widget.clone();
                let duration = self.duration_ms;

                glib::timeout_add_local(Duration::from_millis(duration as u64), move || {
                    widget_clone.set_opacity(0.0);
                    callback();
                    glib::ControlFlow::Break
                });
            }
            _ => {
                // For other transitions, just call the callback
                callback();
            }
        }
    }
}

impl Default for LayoutTransition {
    fn default() -> Self {
        Self::new()
    }
}
