pub mod config;
pub mod graphic;
pub mod mcp;
pub mod personalization;
pub mod widget;

use crate::widget::ButtonWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin_graphic;

widget_plugin_graphic!(ButtonWidget);
