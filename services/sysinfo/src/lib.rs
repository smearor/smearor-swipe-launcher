pub(crate) mod collector;
pub(crate) mod config;
pub(crate) mod service;

use crate::service::SysinfoService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(SysinfoService);
