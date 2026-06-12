use crate::config::launcher::SwipeLauncherConfig;
use crate::plugin_manager::PluginManager;
use crate::service_manager::ServiceManager;
use crate::window::create_window;
use gtk4::Application;
use gtk4::Box as GtkBox;
use gtk4::EventSequenceState;
use gtk4::GestureDrag;
use gtk4::Orientation;
use gtk4::PolicyType;
use gtk4::PropagationPhase;
use gtk4::ScrolledWindow;
use gtk4::Widget;
use gtk4::gdk::Display;
use gtk4::glib::MainContext;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use miette::miette;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_wrot_rotation::RotationWidget;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::channel;
use tracing::error;
use tracing::info;

/// Main application state
pub struct LauncherApplication {
    pub(crate) config: SwipeLauncherConfig,
    pub(crate) plugin_manager: Arc<PluginManager>,
    pub(crate) service_manager: Arc<ServiceManager>,
    pub(crate) message_receiver: std::sync::Mutex<Option<Receiver<FfiEnvelope>>>,
    pub(crate) gtk_app: Application,
}

impl LauncherApplication {}

impl LauncherApplication {
    pub fn new(config: SwipeLauncherConfig, gtk_app: Application) -> Self {
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
        let plugins = Arc::new(self.plugin_manager.plugins.clone());

        let config_inner = config.clone();
        let self_clone = self.clone();
        let Ok(mut receiver) = self.message_receiver.lock() else {
            return Err(miette!("Failed to lock message receiver"));
        };
        let Some(receiver) = receiver.take() else {
            return Err(miette!("Receiver already taken"));
        };
        let receiver_cell = Rc::new(RefCell::new(Some(receiver)));

        let rotation = config.launcher.rotation.clone();
        let layout_config = config.layout.clone();
        self.gtk_app.connect_activate(move |app| {
            info!("GTK application activated");

            if let Some(display) = Display::default() {
                let provider = gtk4::CssProvider::new();
                provider.load_from_string(include_str!("../../resources/style.css"));
                gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            }

            let window = create_window(app, &config_inner);

            let rotation_widget = RotationWidget::new(rotation.rotation());
            // TODO: enable/disable rotation
            rotation_widget.set_animation_speed(rotation.animation_speed());
            rotation_widget.set_animation_overshoot(rotation.animation_overshoot());
            rotation_widget.set_animations_enabled(rotation.animations_enabled());

            let main_container = GtkBox::builder()
                .orientation(Orientation::from(&layout_config.orientation))
                .spacing(layout_config.spacing)
                .build();
            rotation_widget.set_child(Some(&main_container));

            let mut first_scrolled_window: Option<ScrolledWindow> = None;

            for area_id in &config_inner.areas {
                if let Some(area_config) = config_inner.get_area_config(area_id) {
                    let area_widget: Widget = match area_config.area_type {
                        crate::config::area::area_type::AreaType::Fixed => {
                            let width = area_config.width.unwrap_or(200);
                            let box_widget = GtkBox::builder()
                                .orientation(Orientation::Horizontal)
                                .width_request(width)
                                .css_classes(["static-area"])
                                .build();

                            for plugin_entry in &area_config.plugins {
                                if let Some(plugin) = plugins.get(&plugin_entry.id) {
                                    unsafe {
                                        if let Some(ffi_widget) = plugin.build_widget() {
                                            let widget = Widget::from_glib_full(ffi_widget.raw_widget);
                                            box_widget.append(&widget);
                                            info!("Area {} plugin {} successfully added to UI", area_id, plugin_entry.id);
                                        } else {
                                            error!("Area {} plugin {} failed to build widget", area_id, plugin_entry.id);
                                        }
                                    }
                                }
                            }

                            box_widget.upcast()
                        }
                        crate::config::area::area_type::AreaType::Scroll => {
                            let scrolled_window = ScrolledWindow::builder()
                                .hscrollbar_policy(PolicyType::External)
                                .vscrollbar_policy(PolicyType::Never)
                                .hexpand(true)
                                .vexpand(true)
                                .build();

                            if first_scrolled_window.is_none() {
                                first_scrolled_window = Some(scrolled_window.clone());
                            }

                            let plugin_container = GtkBox::builder().orientation(Orientation::Horizontal).spacing(10).build();

                            for plugin_entry in &area_config.plugins {
                                if let Some(plugin) = plugins.get(&plugin_entry.id) {
                                    unsafe {
                                        if let Some(ffi_widget) = plugin.build_widget() {
                                            let widget = Widget::from_glib_full(ffi_widget.raw_widget);
                                            plugin_container.append(&widget);
                                            info!("Area {} plugin {} successfully added to UI", area_id, plugin_entry.id);
                                        } else {
                                            error!("Area {} plugin {} failed to build widget", area_id, plugin_entry.id);
                                        }
                                    }
                                }
                            }

                            scrolled_window.set_child(Some(&plugin_container));

                            let hadjustment = scrolled_window.hadjustment();
                            let drag_gesture = GestureDrag::new();
                            drag_gesture.set_propagation_phase(PropagationPhase::Bubble);
                            let start_value = Arc::new(Cell::new(0.0));
                            let is_dragging = Arc::new(Cell::new(false));
                            const DRAG_THRESHOLD: f64 = 10.0;

                            let start_value_clone1 = start_value.clone();
                            let hadjustment_clone1 = hadjustment.clone();
                            let is_dragging_clone1 = is_dragging.clone();
                            drag_gesture.connect_drag_begin(move |_gesture, _, _| {
                                info!("drag begin");
                                start_value_clone1.set(hadjustment_clone1.value());
                                is_dragging_clone1.set(false);
                            });

                            let start_value_clone2 = start_value.clone();
                            let hadjustment_clone2 = hadjustment.clone();
                            let is_dragging_clone2 = is_dragging.clone();
                            drag_gesture.connect_drag_update(move |gesture, offset_x, _| {
                                if offset_x.abs() > DRAG_THRESHOLD {
                                    info!("drag threshold -> claimed");
                                    gesture.set_state(EventSequenceState::Claimed);
                                    is_dragging_clone2.set(true);
                                }
                                let new_val = start_value_clone2.get() - offset_x;
                                hadjustment_clone2.set_value(new_val);
                            });

                            let is_dragging_clone3 = is_dragging.clone();
                            drag_gesture.connect_drag_end(move |gesture, _, _| {
                                info!("drag end");
                                if !is_dragging_clone3.get() {
                                    info!("denied");
                                    gesture.set_state(EventSequenceState::Denied);
                                }
                            });

                            scrolled_window.add_controller(drag_gesture);
                            scrolled_window.upcast()
                        }
                    };

                    main_container.append(&area_widget);
                }
            }

            let self_clone2 = self_clone.clone();
            if let Some(mut receiver) = receiver_cell.borrow_mut().take() {
                MainContext::default().spawn_local(async move {
                    while let Some(envelope) = receiver.recv().await {
                        if let Some(ref scrolled_window) = first_scrolled_window {
                            self_clone2.handle_message(envelope, scrolled_window);
                        }
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
