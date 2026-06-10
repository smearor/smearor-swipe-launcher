use crate::config::LauncherConfig;
use crate::error::Result;
use crate::plugin::LoadedPlugin;
use crate::plugin_manager::PluginManager;
use crate::window::create_window;
use dashmap::DashMap;
use gtk4::Application;
use gtk4::Box as GtkBox;
use gtk4::GestureDrag;
use gtk4::Orientation;
use gtk4::PolicyType;
use gtk4::ScrolledWindow;
use gtk4::Widget;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use smearor_plugin_api::CoreMessage;
use smearor_plugin_api::PluginConfig;
use smearor_wrot_rotation::RotationWidget;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use tracing::debug;
use tracing::error;
use tracing::info;

/// Main application state
pub struct LauncherApplication {
    pub(crate) config: LauncherConfig,
    pub(crate) plugin_manager: PluginManager,
    pub(crate) message_receiver: Receiver<CoreMessage>,
    pub(crate) gtk_app: Application,
}

impl LauncherApplication {}

impl LauncherApplication {
    pub fn new(config: LauncherConfig, gtk_app: Application) -> Self {
        let (sender, receiver) = mpsc::channel::<CoreMessage>();
        LauncherApplication {
            config,
            plugin_manager: PluginManager::new(sender),
            message_receiver: receiver,
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

    pub fn build_ui(&self, config: &LauncherConfig) {
        let plugins = Arc::new(self.plugin_manager.plugins.clone());
        let rotation = config.launcher.rotation;

        let left_plugin_ids: Vec<String> = config.left_area.plugin_ids();
        let scroll_plugin_ids: Vec<String> = config.scroll_band.plugin_ids();
        let right_plugin_ids: Vec<String> = config.right_area.plugin_ids();

        let config_inner = config.clone();
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
            let start_value = Arc::new(std::cell::Cell::new(0.0));

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
                .orientation(gtk4::Orientation::Horizontal)
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

            window.set_child(Some(&rotation_widget));
            window.present();
        });
    }

    pub fn run(&self) {
        self.gtk_app.run();
    }
}

impl Drop for LauncherApplication {
    fn drop(&mut self) {
        self.plugin_manager.unload_plugins();
    }
}
