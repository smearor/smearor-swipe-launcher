use crate::area::area_manager::AreaManager;
use crate::config::area::config_entry::ConfigEntry;
use crate::config::launcher::SwipeLauncherConfig;
use crate::context::GLOBAL_JSON_CONVERTER_REGISTRY;
use crate::display::AreaSize;
use crate::display::validate_monitor_index;
use crate::json_converter::JsonConverterRegistry;
use crate::plugin_manager::PluginManager;
use crate::window::create_window;
use crate::window::set_anchors_for_rotation;
use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::Box as GtkBox;
use gtk4::Orientation;
use gtk4::prelude::*;
use smearor_model_compositor::MonitorChangeType;
use smearor_model_compositor::WorkspaceLifecycleType;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_wrot_rotation::RotationWidget;
use smearor_wrot_rotation::SmearorRotation;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tracing::error;
use tracing::trace;

/// Per-window launcher instance with isolated plugin and area state.
///
/// Each instance has its own `PluginManager`, `AreaManager`, and window.
/// Messages are sent to the central broker in `LauncherHost`, which routes
/// them to the correct instance using `target_instance_id`.
pub struct LauncherInstance {
    pub(crate) config: SwipeLauncherConfig,
    pub(crate) plugin_manager: Arc<PluginManager>,
    pub(crate) area_manager: Arc<Mutex<AreaManager>>,
    pub(crate) topic_rate_limiter: Arc<Mutex<HashMap<String, Instant>>>,
    pub(crate) window: Mutex<Option<ApplicationWindow>>,
    pub(crate) instance_id: String,
    pub(crate) coordinated_size: Mutex<Option<AreaSize>>,
}

impl LauncherInstance {
    pub fn new(config: SwipeLauncherConfig, instance_id: String, broker_sender: UnboundedSender<FfiEnvelope>) -> Self {
        let plugin_manager = Arc::new(PluginManager::new(broker_sender, instance_id.clone()));
        let config_arc = Arc::new(config.clone());
        let json_converter_registry = GLOBAL_JSON_CONVERTER_REGISTRY
            .get()
            .cloned()
            .unwrap_or_else(|| Arc::new(JsonConverterRegistry::new()));
        let area_manager = Arc::new(Mutex::new(AreaManager::new(plugin_manager.clone(), config_arc, json_converter_registry.clone())));

        LauncherInstance {
            config,
            plugin_manager,
            area_manager,
            topic_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            window: Mutex::new(None),
            instance_id,
            coordinated_size: Mutex::new(None),
        }
    }

    pub fn load_plugins(&self) {
        let monitor_index = self.config.launcher.layer.monitor;
        let (areas, entries) = self.config.get_layout_for_context(None, monitor_index, None);

        for area_id in areas {
            if let Some(ConfigEntry::Area(area_config)) = entries.get(area_id) {
                for plugin_entry in &area_config.plugins {
                    if plugin_entry.disabled {
                        debug!("Skipping disabled plugin {} on area {}", plugin_entry.id, area_id);
                        continue;
                    }
                    trace!("Loading plugin {} on area {}", plugin_entry.id, area_id);
                    let plugin_config = self.config.plugin_config(&plugin_entry.id);
                    trace!("Plugin config: {plugin_config:?}");
                    if let Err(e) = self.plugin_manager.load_plugin(&plugin_entry, plugin_config) {
                        error!("Failed to load plugin {}: {}", plugin_entry.id, e);
                    }
                }
            }
        }
        debug!("Successfully loaded {} plugins", self.plugin_manager.plugins.len());
    }

    pub fn build_window(&self, app: &Application) -> miette::Result<ApplicationWindow> {
        let config = &self.config;
        let launcher_config = config.launcher.clone();
        let rotation = config.launcher.rotation.clone();
        let layout_config = config.layout.clone();

        validate_monitor_index(launcher_config.layer.monitor, &self.instance_id);

        let coordinated_size = self.coordinated_size.lock().ok().and_then(|g| *g);
        let window = create_window(app, &launcher_config, coordinated_size);

        let app_clone = app.clone();
        window.connect_close_request(move |_win| {
            let app = app_clone.clone();
            gtk4::glib::idle_add_local_once(move || {
                app.quit();
            });
            gtk4::glib::Propagation::Proceed
        });

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

        if let Ok(area_manager) = self.area_manager.lock() {
            if let Err(e) = area_manager.set_main_container(main_container) {
                error!("{e}");
            }
        };

        let monitor_index = launcher_config.layer.monitor;
        let (areas, entries) = config.get_layout_for_context(None, monitor_index, None);

        for area_id in areas {
            if let Some(ConfigEntry::Area(area_config)) = entries.get(area_id) {
                let area_manager_clone = self.area_manager.clone();
                let area_id_clone = area_id.clone();
                let area_config_clone = area_config.clone();

                if let Ok(area_manager) = area_manager_clone.lock() {
                    if let Err(e) = area_manager.add_area_from_config(&area_id_clone, area_config_clone) {
                        error!("Failed to add area {}: {}", area_id_clone, e);
                    } else {
                        trace!("Successfully added area {} using AreaManager", area_id_clone);
                    }
                }
            }
        }

        window.set_child(Some(&rotation_widget));
        window.present();

        Ok(window)
    }

    /// Rebuild areas from a resolved layout profile at runtime.
    ///
    /// Clears all existing areas (unloading their plugins) and adds new ones
    /// from the given layout. Used when the layout profile changes due to
    /// monitor hotplug or workspace changes.
    pub fn rebuild_areas(&self, areas: &[String], entries: &HashMap<String, ConfigEntry>) {
        if let Ok(area_manager) = self.area_manager.lock() {
            area_manager.clear_areas();

            for area_id in areas {
                if let Some(ConfigEntry::Area(area_config)) = entries.get(area_id) {
                    if let Err(error) = area_manager.add_area_from_config(area_id, area_config.clone()) {
                        error!("Failed to add area {area_id}: {error}");
                    } else {
                        trace!("Successfully rebuilt area {area_id}");
                    }
                }
            }
        }
    }

    /// Handle a workspace change event from a compositor service.
    ///
    /// Re-evaluates the layout profile with the new workspace ID and monitor
    /// index, then rebuilds areas if the resolved layout differs from the
    /// current one.
    pub fn on_workspace_changed(&self, workspace_id: i32, monitor_index: u32) {
        let (areas, entries) = self.config.get_layout_for_context(None, Some(monitor_index), Some(workspace_id));
        debug!(
            "Instance {} re-evaluating layout for workspace {} on monitor {}",
            self.instance_id, workspace_id, monitor_index
        );
        self.rebuild_areas(areas, entries);
    }

    /// Handle a monitor hotplug event from a compositor service.
    ///
    /// Re-evaluates the monitor mapping and rebuilds areas if the monitor
    /// configuration affects this instance.
    pub fn on_monitor_changed(&self, monitor_index: u32, connector_name: &str, change_type: MonitorChangeType) {
        debug!("Instance {} monitor {} ({}): {:?}", self.instance_id, monitor_index, connector_name, change_type);
        let (areas, entries) = self.config.get_layout_for_context(Some(connector_name), Some(monitor_index), None);
        self.rebuild_areas(areas, entries);
    }

    /// Handle a workspace lifecycle event from a compositor service.
    ///
    /// Currently informational — future widgets may use this to display
    /// workspace lists or update state.
    pub fn on_workspace_lifecycle(&self, workspace_id: i32, monitor_index: u32, lifecycle_type: WorkspaceLifecycleType) {
        debug!("Instance {} workspace {} on monitor {}: {:?}", self.instance_id, workspace_id, monitor_index, lifecycle_type);
    }
}
