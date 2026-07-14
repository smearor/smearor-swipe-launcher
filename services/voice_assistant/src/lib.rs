pub(crate) mod audio;
#[cfg(test)]
pub(crate) mod benchmark;
pub(crate) mod config;
pub(crate) mod gpu_detection;
pub(crate) mod llm;
pub(crate) mod mcp;
pub(crate) mod memory;
pub(crate) mod performance;
pub(crate) mod react;
pub(crate) mod service;
pub(crate) mod tool_cache;
pub(crate) mod tool_catalog;
pub(crate) mod tool_router;
pub(crate) mod transcriber;

use crate::service::VoiceAssistantService;
use smearor_swipe_launcher_plugin_api::service_plugin;

service_plugin!(VoiceAssistantService);
