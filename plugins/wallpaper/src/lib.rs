pub mod config;
pub mod preview;
pub mod widget;

use crate::widget::WallpaperWidget;
use smearor_swipe_launcher_plugin_api::widget_plugin;

widget_plugin!(WallpaperWidget);
