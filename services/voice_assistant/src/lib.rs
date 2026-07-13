pub(crate) mod audio;
pub(crate) mod config;
pub(crate) mod llm;
pub(crate) mod mcp;
pub(crate) mod react;
pub(crate) mod service;
pub(crate) mod tool_catalog;
pub(crate) mod transcriber;

use crate::service::VoiceAssistantService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(VoiceAssistantService);
