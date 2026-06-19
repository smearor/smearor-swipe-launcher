use crate::config::launcher::SwipeLauncherConfig;
use crate::config::services::ServicesConfig;
use crate::css_provider::create_css_provider;
use crate::instance::LauncherInstance;
use crate::service_manager::ServiceManager;
use gtk4::Application;
use gtk4::IconTheme;
use gtk4::gio;
use gtk4::glib::MainContext;
use gtk4::prelude::*;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// Host that manages all launcher instances in a single process.
///
/// Owns the single `gtk4::Application`, the shared `ServiceManager`,
/// the central message broker, and a collection of `LauncherInstance`s.
#[derive(Clone)]
pub struct LauncherHost {
    pub(crate) gtk_app: Application,
    pub(crate) service_manager: Arc<ServiceManager>,
    pub(crate) broker_sender: UnboundedSender<FfiEnvelope>,
    pub(crate) broker_receiver: Arc<Mutex<Option<UnboundedReceiver<FfiEnvelope>>>>,
    pub(crate) instances: Arc<Mutex<HashMap<String, LauncherInstance>>>,
}

impl LauncherHost {
    pub fn new(gtk_app: Application) -> Self {
        let (broker_sender, broker_receiver) = unbounded_channel::<FfiEnvelope>();
        let service_manager = Arc::new(ServiceManager::new(broker_sender.clone()));

        LauncherHost {
            gtk_app,
            service_manager,
            broker_sender,
            broker_receiver: Arc::new(Mutex::new(Some(broker_receiver))),
            instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_instance(&self, instance_id: String, config: SwipeLauncherConfig) {
        let instance = LauncherInstance::new(config, instance_id.clone(), self.broker_sender.clone());
        instance.load_plugins();
        if let Ok(mut instances) = self.instances.lock() {
            instances.insert(instance_id, instance);
        }
    }

    pub fn load_services(&self, services_config: &ServicesConfig) {
        for service_entry in &services_config.services {
            trace!("Loading service {}", service_entry.id);
            let service_config = services_config.plugin_config(&service_entry.id);
            trace!("Service config: {service_config:?}");
            if let Err(e) = self.service_manager.load_service(&service_entry, service_config) {
                error!("Failed to load service {}: {}", service_entry.id, e);
            }
        }
        debug!("Successfully loaded {} services", self.service_manager.services.len());
    }

    pub fn build_ui(&self) -> miette::Result<()> {
        let self_clone = self.clone();

        self.gtk_app.connect_activate(move |app| {
            trace!("GTK application activated");

            create_css_provider();

            match gio::resources_register_include!("compiled.gresource") {
                Ok(_) => {
                    IconTheme::default().add_resource_path("/io/smearor/icons");
                }
                Err(e) => {
                    error!("Failed to register gresource: {e}");
                }
            }

            let instances = if let Ok(instances) = self_clone.instances.lock() {
                instances.values().map(|i| (i.instance_id.clone(), i.build_window(app))).collect::<Vec<_>>()
            } else {
                Vec::new()
            };

            for (instance_id, result) in instances {
                match result {
                    Ok(window) => {
                        if let Ok(instances) = self_clone.instances.lock() {
                            if let Some(instance) = instances.get(&instance_id) {
                                if let Ok(mut w) = instance.window.lock() {
                                    *w = Some(window);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to build window for instance {}: {}", instance_id, e);
                    }
                }
            }
        });

        self.start_broker_loop()?;

        Ok(())
    }

    fn start_broker_loop(&self) -> miette::Result<()> {
        let Ok(mut receiver_guard) = self.broker_receiver.lock() else {
            return Err(miette::miette!("Failed to lock broker receiver"));
        };
        let Some(mut receiver) = receiver_guard.take() else {
            return Err(miette::miette!("Broker receiver already taken"));
        };

        let self_clone = self.clone();
        MainContext::default().spawn_local(async move {
            while let Some(envelope) = receiver.recv().await {
                self_clone.route_message(envelope);
            }
            error!("Central broker receive loop exited");
        });

        Ok(())
    }

    fn route_message(&self, envelope: FfiEnvelope) {
        let mut target = envelope.target_instance_id.to_string();
        let topic = envelope.topic.to_string();

        // Service commands are routed to the shared ServiceManager
        if topic.starts_with("service.") && !topic.ends_with(".status") {
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 {
                let target_service_id = parts[1];
                if let Some(service) = self.service_manager.services.get(target_service_id) {
                    trace!("Routing message to service {}", target_service_id);
                    unsafe {
                        service.on_message(envelope);
                    }
                }
            }
            return;
        }

        // Broadcast to all instances (used by shared services for status updates)
        if target == "*" {
            if let Ok(instances) = self.instances.lock() {
                for instance in instances.values() {
                    instance.handle_message(envelope.clone());
                }
            }
            return;
        }

        // Detect implicit cross-instance addressing when a plugin sends an area_id
        // containing a colon (e.g. "side2:submenu") without setting target_instance_id.
        if target.is_empty() && topic.starts_with("area.") {
            let parts: Vec<&str> = topic.split('.').collect();
            if parts.len() >= 2 && parts[1].contains(':') {
                let (instance, area) = parts[1].split_once(':').unwrap_or(("", ""));
                if !instance.is_empty() {
                    target = instance.to_string();
                    // Reconstruct topic with local area_id for the target instance
                    let new_topic = format!(
                        "area.{}{}",
                        area,
                        if parts.len() > 2 {
                            format!(".{}", &parts[2..].join("."))
                        } else {
                            String::new()
                        }
                    );
                    let mut envelope = envelope;
                    envelope.topic = stabby::string::String::from(new_topic);
                    if let Ok(instances) = self.instances.lock() {
                        if let Some(target_instance) = instances.get(&target) {
                            target_instance.handle_message(envelope);
                        } else {
                            debug!("Unknown target instance '{}' for area message, dropping", target);
                        }
                    }
                    return;
                }
            }
        }

        // Route to a specific instance
        let target_instance = if target.is_empty() {
            // Extract instance from sender_id (format: "instance_id:plugin_id")
            envelope.sender_id.to_string().split(':').next().unwrap_or("").to_string()
        } else {
            target
        };

        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(&target_instance) {
                instance.handle_message(envelope);
            } else {
                debug!("Unknown target instance '{}', dropping message", target_instance);
            }
        }
    }

    pub fn run(&self) {
        self.gtk_app.run_with_args(&[] as &[&str]);
    }
}

impl Drop for LauncherHost {
    fn drop(&mut self) {
        self.service_manager.unload_services();
    }
}
