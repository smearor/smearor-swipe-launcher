pub(crate) mod atomic;
pub(crate) mod config;
pub(crate) mod graphic;
pub(crate) mod html;
pub(crate) mod labels;
pub(crate) mod mcp;
pub(crate) mod personalization;
pub(crate) mod widget;

use crate::atomic::NetworkAtomicWidget;
use crate::widget::NetworkWidget;
use smearor_swipe_launcher_plugin_api::widget_factory_plugin_graphic;

widget_factory_plugin_graphic! {
    "network" => network_widget => NetworkWidget => html,
    "network_wifi_status" => network_wifi_status_widget => NetworkAtomicWidget,
    "network_wifi_connect" => network_wifi_connect_widget => NetworkAtomicWidget,
    "network_ethernet_status" => network_ethernet_status_widget => NetworkAtomicWidget,
    "network_vpn_toggle" => network_vpn_toggle_widget => NetworkAtomicWidget,
}
