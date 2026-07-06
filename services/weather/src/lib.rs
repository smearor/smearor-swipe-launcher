pub(crate) mod config;
pub(crate) mod fetcher;
pub(crate) mod service;

use crate::service::WeatherService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(WeatherService);
