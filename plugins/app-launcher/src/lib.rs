pub mod config;
pub mod widget;

use crate::widget::AppLauncherWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin;

widget_plugin!(AppLauncherWidget);
