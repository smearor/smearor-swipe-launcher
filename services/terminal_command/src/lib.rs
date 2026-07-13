pub(crate) mod config;
pub(crate) mod service;

use crate::service::TerminalCommandService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(TerminalCommandService);
