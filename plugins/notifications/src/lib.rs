pub(crate) mod atomic;
pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub(crate) mod mcp;
pub(crate) mod personalization;
pub(crate) mod widget;

use crate::atomic::NotificationAtomicWidget;
use crate::widget::NotificationWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "notifications" => notifications_widget => NotificationWidget => html,
    "notifications_count" => notifications_count_widget => NotificationAtomicWidget,
    "notifications_latest" => notifications_latest_widget => NotificationAtomicWidget,
    "notifications_dnd" => notifications_dnd_widget => NotificationAtomicWidget,
}
