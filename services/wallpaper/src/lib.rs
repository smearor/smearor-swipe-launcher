pub(crate) mod config;
pub(crate) mod process;
pub(crate) mod service;

use crate::service::WallpaperService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(WallpaperService);
