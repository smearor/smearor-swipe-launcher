pub(crate) mod config;
pub(crate) mod dbus;
pub(crate) mod service;
pub(crate) mod throughput;

use crate::service::NetworkService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(NetworkService);
