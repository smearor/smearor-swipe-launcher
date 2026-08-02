pub mod config;
pub mod graphic;
pub mod html;
pub mod labels;
pub mod mcp;
pub mod personalization;
pub mod widget;

use crate::widget::AppLauncherWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin_graphic;

widget_plugin_graphic!(AppLauncherWidget);
