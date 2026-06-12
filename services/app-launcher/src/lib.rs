pub(crate) mod service;

use crate::service::AppLauncherService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(AppLauncherService);
