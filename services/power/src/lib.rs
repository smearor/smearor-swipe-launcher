pub(crate) mod config;
pub(crate) mod dbus;
pub(crate) mod scheduler;
pub(crate) mod service;

use crate::service::PowerService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(PowerService);
