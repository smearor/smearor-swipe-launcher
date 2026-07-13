pub(crate) mod config;
pub(crate) mod dbus;
pub(crate) mod mcp;
pub(crate) mod mpris_command;
pub(crate) mod mpris_state;
pub(crate) mod service;

use crate::service::MprisService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(MprisService);
