use crate::area::area_manager::AreaManager;
use crate::config::launcher::SwipeLauncherConfig;
use crate::css_provider::create_css_provider;
use crate::plugin_manager::PluginManager;
use crate::service_manager::ServiceManager;
use crate::window::create_window;
use crate::window::set_anchors_for_rotation;
use gtk4::Application;
use gtk4::Box as GtkBox;
use gtk4::IconTheme;
use gtk4::Orientation;
use gtk4::ScrolledWindow;
use gtk4::gio;
use gtk4::glib::MainContext;
use gtk4::prelude::*;
use miette::miette;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::channel;
use tracing::error;
use tracing::info;

/// Main application state
pub struct LauncherApplication {
    pub(crate) config: SwipeLauncherConfig,
    pub(crate) plugin_manager: Arc<PluginManager>,
    pub(crate) service_manager: Arc<ServiceManager>,
    pub(crate) area_manager: Arc<Mutex<AreaManager>>,
    pub(crate) message_receiver: Mutex<Option<Receiver<FfiEnvelope>>>,
    pub(crate) gtk_app: Application,
}

impl LauncherApplication {}

impl LauncherApplication {
    pub fn new(config: SwipeLauncherConfig, gtk_app: Application) -> Self {
        let (sender, receiver) = channel::<FfiEnvelope>(100);
        let plugin_manager = Arc::new(PluginManager::new(sender.clone()));
        let config_arc = Arc::new(config.clone());
        let area_manager = Arc::new(Mutex::new(AreaManager::new(plugin_manager.clone(), config_arc)));

        LauncherApplication {
            config,
            plugin_manager,
            service_manager: Arc::new(ServiceManager::new(sender)),
            area_manager,
            message_receiver: Mutex::new(Some(receiver)),
            gtk_app,
        }
    }

    pub fn load_plugins(&self) {
        for area_id in &self.config.areas {
            if let Some(area_config) = self.config.get_area_config(area_id) {
                for plugin_entry in &area_config.plugins {
                    info!("Loading plugin {} on area {}", plugin_entry.id, area_id);
                    let plugin_config = self.config.plugin_config(&plugin_entry.id);
                    info!("Plugin config: {plugin_config:?}");
                    if let Err(e) = self.plugin_manager.load_plugin(&plugin_entry, plugin_config) {
                        error!("Failed to load plugin {}: {}", plugin_entry.id, e);
                    }
                }
            }
        }
        info!("Successfully loaded {} plugins", self.plugin_manager.plugins.len());
    }

    pub fn load_services(&self) {
        for service_entry in &self.config.services {
            info!("Loading service {}", service_entry.id);
            let service_config = self.config.plugin_config(&service_entry.id);
            info!("Service config: {service_config:?}");
            if let Err(e) = self.service_manager.load_service(&service_entry, service_config) {
                error!("Failed to load service {}: {}", service_entry.id, e);
            }
        }
        info!("Successfully loaded {} services", self.service_manager.services.len());
    }

    pub fn build_ui(self: Arc<Self>, config: &SwipeLauncherConfig) -> miette::Result<()> {
        let self_clone = self.clone();
        let Ok(mut receiver) = self.message_receiver.lock() else {
            return Err(miette!("Failed to lock message receiver"));
        };
        let Some(receiver) = receiver.take() else {
            return Err(miette!("Receiver already taken"));
        };
        let receiver_cell = Rc::new(RefCell::new(Some(receiver)));

        let config_inner = config.clone();
        let launcher_config = config.launcher.clone();
        let rotation = config.launcher.rotation.clone();
        let layout_config = config.layout.clone();
        self.gtk_app.connect_activate(move |app| {
            info!("GTK application activated");

            create_css_provider();

            match gio::resources_register_include!("compiled.gresource") {
                Ok(_) => {
                    IconTheme::default().add_resource_path("/io/smearor/icons");
                }
                Err(e) => {
                    error!("Failed to register gresource: {e}");
                }
            }

            let window = create_window(app, &launcher_config);

            let rotation_widget = RotationWidget::new(rotation.rotation());
            rotation_widget.set_animation_speed(rotation.animation_speed());
            rotation_widget.set_animation_overshoot(rotation.animation_overshoot());
            rotation_widget.set_animations_enabled(rotation.animations_enabled());

            let window_weak = window.downgrade();
            rotation_widget.connect_notify_local(Some("rotation"), move |widget, _| {
                if let Some(win) = window_weak.upgrade() {
                    let degrees: f32 = widget.property("rotation");
                    let rotation = SmearorRotation::new(degrees);
                    set_anchors_for_rotation(&win, rotation);
                }
            });

            let main_container = GtkBox::builder()
                .orientation(Orientation::from(&layout_config.orientation))
                .spacing(layout_config.spacing)
                .build();

            rotation_widget.set_child(Some(&main_container));

            if let Ok(area_manager) = self_clone.area_manager.lock() {
                if let Err(e) = area_manager.set_main_container(main_container) {
                    error!("{e}");
                }
            };

            let mut first_scrolled_window: Option<ScrolledWindow> = None;

            for area_id in &config_inner.areas {
                if let Some(area_config) = config_inner.get_area_config(area_id) {
                    let area_manager_clone = self_clone.area_manager.clone();
                    let area_id_clone = area_id.clone();
                    let area_config_clone = area_config.clone();

                    if let Ok(area_manager) = area_manager_clone.lock() {
                        if let Err(e) = area_manager.add_area_from_config(&area_id_clone, area_config_clone) {
                            error!("Failed to add area {}: {}", area_id_clone, e);
                        } else {
                            info!("Successfully added area {} using AreaManager", area_id_clone);
                            if first_scrolled_window.is_none() {
                                if let Some(scrolled_window) = area_manager.get_first_scrolled_window(&area_id_clone) {
                                    first_scrolled_window = Some(scrolled_window);
                                }
                            }
                        }
                    }
                }
            }

            let self_clone2 = self_clone.clone();
            if let Some(mut receiver) = receiver_cell.borrow_mut().take() {
                MainContext::default().spawn_local(async move {
                    while let Some(envelope) = receiver.recv().await {
                        self_clone2.handle_message(envelope);
                        // if let Some(ref scrolled_window) = first_scrolled_window {
                        //     // self_clone2.handle_message(envelope, scrolled_window, &main_container_clone);
                        // }
                    }
                });
            }

            window.set_child(Some(&rotation_widget));
            window.present();
        });
        Ok(())
    }

    pub fn run(&self) {
        self.gtk_app.run();
    }
}

impl Drop for LauncherApplication {
    fn drop(&mut self) {
        self.plugin_manager.unload_plugins();
        self.service_manager.unload_services();
    }
}
