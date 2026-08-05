pub(crate) mod config;
pub(crate) mod mcp;
pub(crate) mod service;
pub(crate) mod state;
pub(crate) mod usb;

use crate::service::DoaService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(DoaService);
