pub(crate) mod config;
pub(crate) mod service;
pub(crate) mod whitelist;

use crate::service::HttpService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(HttpService);
