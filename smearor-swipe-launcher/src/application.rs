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

            let main_container = GtkBox::builder().orientation(Orientation::Horizontal).spacing(0).build();

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

                            let plugin_instance = plugin.instance;
                            let plugin_vtable = plugin.vtable;

                            let click_gesture = GestureClick::builder().button(1).build();
                            click_gesture.connect_pressed(move |gesture, _n_press, _x, _y| {
                                gesture.set_state(EventSequenceState::Claimed);
                                let result = (plugin_vtable.get().on_primary_action)(plugin_instance, rotation);
                                info!("Primary action result: {}", result);
                            });
                            widget.add_controller(click_gesture);

                            let longpress_gesture = gtk4::GestureLongPress::builder().build();
                            longpress_gesture.connect_pressed(move |gesture, _x, _y| {
                                gesture.set_state(EventSequenceState::Claimed);
                                let result = (plugin_vtable.get().on_secondary_action)(plugin_instance, rotation);
                                info!("Secondary action result: {}", result);
                            });
                            widget.add_controller(longpress_gesture);

                            let rotation_str = rotation.to_string();
                            let rotation_widget = RotationWidget::new(SmearorRotation::from(rotation_str.as_str()));
                            rotation_widget.set_width_request(200);
                            rotation_widget.set_child(Some(&widget));

                            left_area.append(&rotation_widget);
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

                            let plugin_instance = plugin.instance;
                            let plugin_vtable = plugin.vtable;

                            let click_gesture = GestureClick::builder().button(1).build();
                            click_gesture.connect_pressed(move |gesture, _n_press, _x, _y| {
                                gesture.set_state(EventSequenceState::Claimed);
                                let result = (plugin_vtable.get().on_primary_action)(plugin_instance, rotation);
                                info!("Primary action result: {}", result);
                            });
                            widget.add_controller(click_gesture);

                            let longpress_gesture = gtk4::GestureLongPress::builder().build();
                            longpress_gesture.connect_pressed(move |gesture, _x, _y| {
                                gesture.set_state(gtk4::EventSequenceState::Claimed);
                                let result = (plugin_vtable.get().on_secondary_action)(plugin_instance, rotation);
                                info!("Secondary action result: {}", result);
                            });
                            widget.add_controller(longpress_gesture);

                            let rotation_str = rotation.to_string();
                            let rotation_widget = RotationWidget::new(SmearorRotation::from(rotation_str.as_str()));
                            rotation_widget.set_width_request(200);
                            rotation_widget.set_child(Some(&widget));

                            plugin_container.append(&rotation_widget);
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

                            let plugin_instance = plugin.instance;
                            let plugin_vtable = plugin.vtable;

                            let click_gesture = gtk4::GestureClick::builder().button(1).build();
                            click_gesture.connect_pressed(move |gesture, _n_press, _x, _y| {
                                gesture.set_state(gtk4::EventSequenceState::Claimed);
                                let result = (plugin_vtable.get().on_primary_action)(plugin_instance, rotation);
                                info!("Primary action result: {}", result);
                            });
                            widget.add_controller(click_gesture);

                            let longpress_gesture = gtk4::GestureLongPress::builder().build();
                            longpress_gesture.connect_pressed(move |gesture, _x, _y| {
                                gesture.set_state(gtk4::EventSequenceState::Claimed);
                                let result = (plugin_vtable.get().on_secondary_action)(plugin_instance, rotation);
                                info!("Secondary action result: {}", result);
                            });
                            widget.add_controller(longpress_gesture);

                            let rotation_str = rotation.to_string();
                            let rotation_widget = RotationWidget::new(SmearorRotation::from(rotation_str.as_str()));
                            rotation_widget.set_width_request(200);
                            rotation_widget.set_child(Some(&widget));

                            right_area.append(&rotation_widget);
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

            window.set_child(Some(&main_container));
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
