pub mod config;
pub mod monitor;
pub mod service;
pub mod workspace;

use crate::service::WaylandWorkspaceService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(WaylandWorkspaceService);
