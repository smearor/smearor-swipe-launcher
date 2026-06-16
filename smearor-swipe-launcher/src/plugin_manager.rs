use crate::error::LauncherError;
use crate::plugin::LoadedPlugin;
use dashmap::DashMap;
use dashmap::DashSet;
use smearor_model_plugin::PluginEntry;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tracing::trace;

pub struct PluginManager {
    pub(crate) plugins: DashMap<String, LoadedPlugin>,
    pub(crate) message_sender: UnboundedSender<FfiEnvelope>,
}

impl PluginManager {
    pub fn new(message_sender: UnboundedSender<FfiEnvelope>) -> Self {
        PluginManager {
            plugins: DashMap::new(),
            message_sender,
        }
    }

    pub fn get_plugin_ids(&self) -> DashSet<String> {
        self.plugins.iter().map(|id| id.key().to_string()).collect()
    }

    pub fn load_plugin(&self, plugin_entry: &PluginEntry, config: PluginConfig) -> Result<(), LauncherError> {
        trace!("Loading plugin {} from: {:?}", plugin_entry.id, plugin_entry.path);

        let (actual_plugin_id, plugin) = LoadedPlugin::load(plugin_entry, &config, self.message_sender.clone())?;

        self.plugins.insert(actual_plugin_id.clone(), plugin);
        debug!("Successfully loaded plugin: {}", actual_plugin_id);

        Ok(())
    }

    pub fn unload_plugin(&self, plugin_id: &str) {
        if let Some((id, plugin)) = self.plugins.remove(plugin_id) {
            unsafe {
                plugin.destroy();
            }
            trace!("Successfully unloaded plugin {id}")
        }
    }

    pub fn unload_plugins(&self) {
        trace!("Cleaning up plugins");

        for id in self.get_plugin_ids().iter() {
            trace!("Destroying plugin: {}", id.as_str());
            self.unload_plugin(id.as_str());
        }
    }
}
