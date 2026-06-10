use crate::config::PluginEntry;
use crate::service::LoadedService;
use dashmap::DashMap;
use dashmap::DashSet;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;
use tracing::debug;
use tracing::info;

pub struct ServiceManager {
    pub(crate) services: DashMap<String, LoadedService>,
    pub(crate) message_sender: Sender<FfiEnvelope>,
}

impl ServiceManager {
    pub fn new(message_sender: Sender<FfiEnvelope>) -> Self {
        ServiceManager {
            services: DashMap::new(),
            message_sender,
        }
    }

    pub fn get_service_ids(&self) -> DashSet<String> {
        self.services.iter().map(|id| id.key().to_string()).collect()
    }

    pub fn load_service(&self, service_entry: &PluginEntry, config: PluginConfig) -> crate::error::Result<()> {
        info!("Loading service {} from: {:?}", service_entry.id, service_entry.path);

        let (actual_service_id, service) = LoadedService::load(service_entry, &config, self.message_sender.clone())?;

        self.services.insert(actual_service_id.clone(), service);
        info!("Successfully loaded service: {}", actual_service_id);

        Ok(())
    }

    pub fn unload_service(&self, service_id: &str) {
        if let Some((id, service)) = self.services.remove(service_id) {
            unsafe {
                service.destroy();
            }
            info!("Successfully unloaded service {id}")
        }
    }

    pub fn unload_services(&self) {
        info!("Cleaning up services");

        for id in self.get_service_ids().iter() {
            debug!("Destroying service: {}", id.as_str());
            self.unload_service(id.as_str());
        }
    }
}
