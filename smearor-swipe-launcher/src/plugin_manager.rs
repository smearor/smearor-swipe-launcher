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
    pub(crate) instance_id: String,
}

impl PluginManager {
    pub fn new(message_sender: UnboundedSender<FfiEnvelope>, instance_id: String) -> Self {
        PluginManager {
            plugins: DashMap::new(),
            message_sender,
            instance_id,
        }
    }

    pub fn get_plugin_ids(&self) -> DashSet<String> {
        self.plugins.iter().map(|id| id.key().to_string()).collect()
    }

    /// Returns the namespaced plugin ID for a raw plugin ID.
    pub fn namespaced_plugin_id(&self, plugin_id: &str) -> String {
        if self.instance_id.is_empty() {
            plugin_id.to_string()
        } else {
            format!("{}:{}", self.instance_id, plugin_id)
        }
    }

    pub fn load_plugin(&self, plugin_entry: &PluginEntry, config: PluginConfig) -> Result<(), LauncherError> {
        trace!("Loading plugin {} from: {:?}", plugin_entry.id, plugin_entry.path);

        let (plugin_id, plugin) = LoadedPlugin::load(plugin_entry, &config, self.message_sender.clone(), &self.instance_id)?;

        let namespaced_id = if self.instance_id.is_empty() {
            plugin_id
        } else {
            format!("{}:{}", self.instance_id, plugin_id)
        };
        self.plugins.insert(namespaced_id.clone(), plugin);
        debug!("Successfully loaded plugin: {}", namespaced_id);

        Ok(())
    }

    pub fn unload_plugin(&self, plugin_id: &str) {
        if let Some((id, plugin)) = self.plugins.remove(plugin_id) {
            unsafe {
                plugin.destroy();
            }
            // Prevent LoadedPlugin::drop from running — it would unload the
            // .so library while the plugin's worker thread (e.g. clock) is
            // still executing code from it. Leaking is safe because
            // std::process::exit(0) follows shortly after shutdown.
            std::mem::forget(plugin);
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
