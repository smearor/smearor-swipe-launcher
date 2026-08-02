pub(crate) mod atomic;
pub mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub mod mcp;
pub(crate) mod personalization;
pub mod preview;
pub mod widget;

use crate::atomic::WallpaperAtomicWidget;
use crate::widget::WallpaperWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "wallpaper" => wallpaper_widget => WallpaperWidget => html,
    "wallpaper_selector" => wallpaper_selector_widget => WallpaperAtomicWidget,
    "wallpaper_next" => wallpaper_next_widget => WallpaperAtomicWidget,
    "wallpaper_previous" => wallpaper_previous_widget => WallpaperAtomicWidget,
    "wallpaper_random" => wallpaper_random_widget => WallpaperAtomicWidget,
    "wallpaper_current" => wallpaper_current_widget => WallpaperAtomicWidget,
}
