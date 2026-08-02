pub(crate) mod command;
pub(crate) mod config;
pub(crate) mod mcp;
pub(crate) mod process;
pub(crate) mod service;
pub(crate) mod state;

use crate::service::WallpaperService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(WallpaperService);
