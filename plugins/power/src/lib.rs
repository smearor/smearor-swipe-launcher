pub(crate) mod atomic;
pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub(crate) mod mcp;
pub(crate) mod personalization;
pub(crate) mod widget;

use crate::atomic::PowerAtomicWidget;
use crate::widget::PowerWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "power" => power_widget => PowerWidget => html,
    "power_shutdown" => power_shutdown_widget => PowerAtomicWidget,
    "power_reboot" => power_reboot_widget => PowerAtomicWidget,
    "power_suspend" => power_suspend_widget => PowerAtomicWidget,
    "power_hibernate" => power_hibernate_widget => PowerAtomicWidget,
    "power_lock" => power_lock_widget => PowerAtomicWidget,
    "power_logout" => power_logout_widget => PowerAtomicWidget,
    "power_standby" => power_standby_widget => PowerAtomicWidget,
}
