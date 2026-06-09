use crate::config::LauncherConfig;
use crate::error::Result;
use crate::plugin::LoadedPlugin;
use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::Box as GtkBox;
use gtk4::EventSequenceState;
use gtk4::GestureClick;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use smearor_plugin_api::CoreMessage;
use smearor_plugin_api::PluginConfig;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use tracing::debug;
use tracing::error;
use tracing::info;

/// Main application state
pub struct LauncherApplication {
    pub(crate) rotation: SmearorRotation,
    pub(crate) plugins: HashMap<String, LoadedPlugin>,
    pub(crate) message_sender: mpsc::Sender<CoreMessage>,
    pub(crate) message_receiver: mpsc::Receiver<CoreMessage>,
    pub(crate) gtk_app: Application,
}

impl LauncherApplication {
    pub fn new(rotation: SmearorRotation, gtk_app: Application) -> Self {
        let (sender, receiver) = mpsc::channel::<CoreMessage>();

        LauncherApplication {
            rotation,
            plugins: HashMap::new(),
            message_sender: sender,
            message_receiver: receiver,
            gtk_app,
        }
    }

    pub fn load_plugin(&mut self, plugin_id: String, plugin_path: PathBuf, config: PluginConfig) -> Result<()> {
        info!("Loading plugin {} from: {:?}", plugin_id, plugin_path);

        let (_, plugin) = LoadedPlugin::load(&plugin_path, &config, self.message_sender.clone())?;

        self.plugins.insert(plugin_id.clone(), plugin);
        info!("Successfully loaded plugin: {}", plugin_id);

        Ok(())
    }

    pub fn build_ui(&self, config: &LauncherConfig) {
        let plugins = Arc::new(self.plugins.clone());
        let rotation = config.launcher.rotation;

        let left_plugin_ids: Vec<String> = config.left_area.plugins.iter().map(|p| p.id.clone()).collect();
        let scroll_plugin_ids: Vec<String> = config.scroll_band.plugins.iter().map(|p| p.id.clone()).collect();
        let right_plugin_ids: Vec<String> = config.right_area.plugins.iter().map(|p| p.id.clone()).collect();

        self.gtk_app.connect_activate(move |app| {
            info!("GTK application activated");

            let window = ApplicationWindow::builder()
                .application(app)
                .title("Smearor Swipe Launcher")
                .default_width(800)
                .default_height(100)
                .build();

            let rotation_widget = RotationWidget::new(SmearorRotation::Deg(rotation));
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

            center_area.append(&plugin_container);

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
        info!("Cleaning up plugins");

        for (id, plugin) in self.plugins.drain() {
            debug!("Destroying plugin: {}", id);

            unsafe {
                plugin.destroy();
            }
        }
    }
}
