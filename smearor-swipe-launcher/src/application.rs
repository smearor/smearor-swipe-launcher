use crate::config::LauncherConfig;
use crate::plugin_manager::PluginManager;
use crate::service_manager::ServiceManager;
use crate::window::create_window;
use gtk4::Application;
use gtk4::Box as GtkBox;
use gtk4::GestureDrag;
use gtk4::Orientation;
use gtk4::PolicyType;
use gtk4::ScrolledWindow;
use gtk4::Widget;
use gtk4::glib::MainContext;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use miette::miette;
use smearor_plugin_api::FfiEnvelope;
use smearor_wrot_rotation::RotationWidget;
use std::cell::Cell;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::channel;
use tracing::error;
use tracing::info;

/// Main application state
pub struct LauncherApplication {
    pub(crate) config: LauncherConfig,
    pub(crate) plugin_manager: Arc<PluginManager>,
    pub(crate) service_manager: Arc<ServiceManager>,
    pub(crate) message_receiver: std::sync::Mutex<Option<Receiver<FfiEnvelope>>>,
    pub(crate) gtk_app: Application,
}

impl LauncherApplication {}

impl LauncherApplication {
    pub fn new(config: LauncherConfig, gtk_app: Application) -> Self {
        let (sender, receiver) = channel::<FfiEnvelope>(100);
        LauncherApplication {
            config,
            plugin_manager: Arc::new(PluginManager::new(sender.clone())),
            service_manager: Arc::new(ServiceManager::new(sender)),
            message_receiver: std::sync::Mutex::new(Some(receiver)),
            gtk_app,
        }
    }

    pub fn load_plugins(&self) {
        for plugin_entry in &self.config.left_area.plugins {
            let plugin_config = self.config.plugin_config(&plugin_entry.id);
            let plugin_path = PathBuf::from(&plugin_entry.path);
            if let Err(e) = self.plugin_manager.load_plugin(plugin_entry.id.clone(), plugin_path, plugin_config) {
                error!("Failed to load plugin {}: {}", plugin_entry.id, e);
            }
        }

        for plugin_entry in &self.config.scroll_band.plugins {
            let plugin_config = self.config.plugin_config(&plugin_entry.id);
            let plugin_path = PathBuf::from(&plugin_entry.path);
            if let Err(e) = self.plugin_manager.load_plugin(plugin_entry.id.clone(), plugin_path, plugin_config) {
                error!("Failed to load plugin {}: {}", plugin_entry.id, e);
            }
        }

        for plugin_entry in &self.config.right_area.plugins {
            let plugin_config = self.config.plugin_config(&plugin_entry.id);
            let plugin_path = PathBuf::from(&plugin_entry.path);
            if let Err(e) = self.plugin_manager.load_plugin(plugin_entry.id.clone(), plugin_path, plugin_config) {
                error!("Failed to load plugin {}: {}", plugin_entry.id, e);
            }
        }
    }

    pub fn load_services(&self) {
        for service_entry in &self.config.services {
            let service_config = self.config.plugin_config(&service_entry.id);
            let service_path = PathBuf::from(&service_entry.path);
            if let Err(e) = self.service_manager.load_service(service_entry.id.clone(), service_path, service_config) {
                error!("Failed to load service {}: {}", service_entry.id, e);
            }
        }
    }

    pub fn build_ui(self: Arc<Self>, config: &LauncherConfig) -> miette::Result<()> {
        let plugins = Arc::new(self.plugin_manager.plugins.clone());
        let rotation = config.launcher.rotation;

        let left_plugin_ids: Vec<String> = config.left_area.plugin_ids();
        let scroll_plugin_ids: Vec<String> = config.scroll_band.plugin_ids();
        let right_plugin_ids: Vec<String> = config.right_area.plugin_ids();

        let config_inner = config.clone();
        let self_clone = self.clone();
        let Ok(mut receiver) = self.message_receiver.lock() else {
            return Err(miette!("Failed to lock message receiver"));
        };
        let Some(receiver) = receiver.take() else {
            return Err(miette!("Receiver already taken"));
        };
        let receiver_cell = Rc::new(RefCell::new(Some(receiver)));

        self.gtk_app.connect_activate(move |app| {
            info!("GTK application activated");

            let window = create_window(app, &config_inner);

            let rotation_widget = RotationWidget::new(rotation);
            rotation_widget.set_animation_speed(500);
            rotation_widget.set_animations_enabled(!true);
            // rotation_widget.set_animation_overshoot(20);

            let main_container = GtkBox::builder().orientation(Orientation::Horizontal).spacing(0).build();
            rotation_widget.set_child(Some(&main_container));

            let left_area = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .width_request(200)
                .css_classes(["static-area"])
                .build();

            for id in &left_plugin_ids {
                if let Some(plugin) = plugins.get(id) {
                    unsafe {
                        if let Some(ffi_widget) = plugin.build_widget() {
                            let widget = Widget::from_glib_full(ffi_widget.raw_widget);
                            left_area.append(&widget);
                            info!("Left area plugin-Widget successfully added to UI");
                        } else {
                            error!("Left area plugin failed to build widget");
                        }
                    }
                }
            }

            let center_area = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .hexpand(true)
                .css_classes(["scroll-area"])
                .build();

            let scrolled_window = ScrolledWindow::builder()
                .hscrollbar_policy(PolicyType::External)
                .vscrollbar_policy(PolicyType::Never)
                .hexpand(true)
                .vexpand(true)
                .build();

            let plugin_container = GtkBox::builder().orientation(Orientation::Horizontal).spacing(10).build();

            for id in &scroll_plugin_ids {
                if let Some(plugin) = plugins.get(id) {
                    unsafe {
                        if let Some(ffi_widget) = plugin.build_widget() {
                            let widget = Widget::from_glib_full(ffi_widget.raw_widget);
                            plugin_container.append(&widget);
                            info!("Scroll band plugin-Widget successfully added to UI");
                        } else {
                            error!("Scroll band plugin failed to build widget");
                        }
                    }
                }
            }

            scrolled_window.set_child(Some(&plugin_container));

            // Custom drag to scroll support with mouse on desktop
            let hadjustment = scrolled_window.hadjustment();
            let drag_gesture = GestureDrag::new();
            let start_value = Arc::new(Cell::new(0.0));

            let start_value_clone1 = start_value.clone();
            let hadjustment_clone1 = hadjustment.clone();
            drag_gesture.connect_drag_begin(move |_, _, _| {
                start_value_clone1.set(hadjustment_clone1.value());
            });

            let start_value_clone2 = start_value.clone();
            let hadjustment_clone2 = hadjustment.clone();
            drag_gesture.connect_drag_update(move |_, offset_x, _| {
                let new_val = start_value_clone2.get() - offset_x;
                hadjustment_clone2.set_value(new_val);
            });

            scrolled_window.add_controller(drag_gesture);

            center_area.append(&scrolled_window);

            let right_area = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .width_request(200)
                .css_classes(["static-area"])
                .build();

            for id in &right_plugin_ids {
                if let Some(plugin) = plugins.get(id) {
                    unsafe {
                        if let Some(ffi_widget) = plugin.build_widget() {
                            let widget = Widget::from_glib_full(ffi_widget.raw_widget);
                            widget.set_width_request(200);
                            right_area.append(&widget);
                            info!("Right area plugin-Widget successfully added to UI");
                        } else {
                            error!("Right area plugin failed to build widget");
                        }
                    }
                }
            }

            main_container.append(&left_area);
            main_container.append(&center_area);
            main_container.append(&right_area);

            // Set up our FfiEnvelope message receiver on the main context loop
            let self_clone2 = self_clone.clone();
            let scrolled_window_clone = scrolled_window.clone();
            if let Some(mut receiver) = receiver_cell.borrow_mut().take() {
                MainContext::default().spawn_local(async move {
                    while let Some(envelope) = receiver.recv().await {
                        self_clone2.handle_message(envelope, &scrolled_window_clone);
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
