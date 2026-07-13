pub(crate) mod config;
pub(crate) mod mcp;
pub(crate) mod pulse;
pub(crate) mod pulse_command;
pub(crate) mod pulse_state;
pub(crate) mod service;

use crate::service::AudioService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(AudioService);
