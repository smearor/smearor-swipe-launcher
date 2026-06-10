use crate::service::LoadedService;
use dashmap::DashMap;
use dashmap::DashSet;
use smearor_plugin_api::FfiEnvelope;
use smearor_plugin_api::PluginConfig;
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

    pub fn load_service(&self, service_id: String, service_path: PathBuf, config: PluginConfig) -> crate::error::Result<()> {
        info!("Loading service {} from: {:?}", service_id, service_path);

        let (_, service) = LoadedService::load(&service_path, &config, self.message_sender.clone())?;

        self.services.insert(service_id.clone(), service);
        info!("Successfully loaded service: {}", service_id);

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
