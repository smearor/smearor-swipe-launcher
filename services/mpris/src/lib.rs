pub(crate) mod config;
pub(crate) mod service;

use crate::service::MprisService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(MprisService);
