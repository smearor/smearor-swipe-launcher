pub mod clock;
pub mod config;
pub mod widget;

use crate::widget::ClockWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin;

widget_plugin!(ClockWidget);
