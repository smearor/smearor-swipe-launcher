pub mod config;
pub mod service;

use crate::service::HyprlandService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(HyprlandService);
