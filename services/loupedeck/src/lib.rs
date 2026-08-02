pub(crate) mod config;
pub(crate) mod mcp;
pub(crate) mod service;

use crate::service::LoupedeckService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(LoupedeckService);
