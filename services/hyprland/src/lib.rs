pub mod config;
pub mod monitor;
pub mod service;
pub mod workspace;

use crate::service::HyprlandService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(HyprlandService);
