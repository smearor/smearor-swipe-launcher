pub mod config;
pub mod desktop_entry;
pub mod widget;

use crate::widget::AppLauncherWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin;

widget_plugin!(AppLauncherWidget);
