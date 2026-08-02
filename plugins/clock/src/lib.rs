pub(crate) mod atomic;
pub mod clock;
pub mod config;
pub mod countdown_state;
pub mod graphic;
pub mod html;
pub mod labels;
pub mod localized_weekday;
pub mod mcp;
pub mod span_state;
pub mod timer_state;
pub mod widget;

use crate::atomic::ClockAtomicWidget;
use crate::widget::ClockWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "clock" => clock_widget => ClockWidget => html,
    "clock_time_digital" => clock_time_digital_widget => ClockAtomicWidget,
    "clock_date_digital" => clock_date_digital_widget => ClockAtomicWidget,
    "clock_time_analog" => clock_time_analog_widget => ClockAtomicWidget,
    "clock_big_digital" => clock_big_digital_widget => ClockAtomicWidget,
    "clock_big_date" => clock_big_date_widget => ClockAtomicWidget,
    "clock_countdown" => clock_countdown_widget => ClockAtomicWidget,
    "clock_timer" => clock_timer_widget => ClockAtomicWidget,
}
