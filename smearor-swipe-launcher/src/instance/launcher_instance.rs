use crate::area::container::HeadlessContainer;
use crate::area::instance_area_manager::InstanceAreaManager;
use crate::config::area::config_entry::ConfigEntry;
use crate::config::launcher::SwipeLauncherConfig;
use crate::context::GLOBAL_JSON_CONVERTER_REGISTRY;
use crate::css::apply_global_scaled_css;
use crate::display::AreaSize;
use crate::display::validate_monitor_index;
use crate::instance::instance_type::InstanceType;
use crate::plugin::PluginManager;
use crate::window::create_window;
use crate::window::set_anchors_for_rotation;
use gtk4::Application;
use gtk4::ApplicationWindow;
use gtk4::Box as GtkBox;
use gtk4::Orientation;
use gtk4::glib::SignalHandlerId;
use gtk4::prelude::*;
use smearor_mcp_server::LogBuffer;
use smearor_model_compositor::MonitorChangeType;
use smearor_model_compositor::WorkspaceLifecycleType;
use smearor_model_instance_control::LauncherInstanceLifecycle;
use smearor_model_macropad::MacroPadDeviceMetadata;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
use smearor_swipe_launcher_plugin_api::sanitize_css_class_name;
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
    pub(crate) config_path: Mutex<Option<String>>,
    pub(crate) plugin_manager: Arc<PluginManager>,
    pub(crate) area_manager: Arc<Mutex<InstanceAreaManager>>,
    pub(crate) topic_rate_limiter: Arc<Mutex<HashMap<String, Instant>>>,
    pub(crate) window: Mutex<Option<ApplicationWindow>>,
    pub(crate) instance_id: String,
    pub(crate) instance_type: InstanceType,
    pub(crate) coordinated_size: Mutex<Option<AreaSize>>,
    pub(crate) device_metadata: Mutex<Option<MacroPadDeviceMetadata>>,
    /// Maps physical button index → plugin ID for MacroPad input dispatch.
    ///
    /// Built during `render_buttons_to_device` to account for 2D span group
    /// alignment shifts. Index `None` means the button is empty/cleared.
    pub(crate) button_map: Mutex<Option<Vec<Option<String>>>>,
    pub(crate) close_handler_id: Mutex<Option<SignalHandlerId>>,
    /// Current lifecycle state of this instance.
    pub(crate) lifecycle: Mutex<LauncherInstanceLifecycle>,
    /// Optional auto-stop TTL timer task handle.
    /// When `auto_stop_ttl` is set, a `tokio::spawn` task is created that
    /// sleeps for the TTL duration and then calls `stop_instance`.
    /// This handle is used to cancel the timer if the instance is stopped
    /// or unloaded before the TTL expires, or if the instance is re-started.
    pub(crate) auto_stop_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl LauncherInstance {
    pub fn new(
        config: SwipeLauncherConfig,
        instance_id: String,
        instance_type: InstanceType,
        broker_sender: UnboundedSender<FfiEnvelope>,
        log_buffer: Option<Arc<LogBuffer>>,
    ) -> Self {
        let plugin_manager = Arc::new(PluginManager::new(broker_sender, instance_id.clone(), log_buffer));
        let config_arc = Arc::new(config.clone());
        let json_converter_registry = GLOBAL_JSON_CONVERTER_REGISTRY
            .get()
            .cloned()
            .unwrap_or_else(|| Arc::new(JsonConverterRegistry::new()));
        let area_manager = if instance_type == InstanceType::Gtk {
            InstanceAreaManager::new_gtk(plugin_manager.clone(), config_arc, json_converter_registry.clone())
        } else {
            InstanceAreaManager::new_headless(plugin_manager.clone(), config_arc, json_converter_registry.clone())
        };
        let area_manager = Arc::new(Mutex::new(area_manager));

        LauncherInstance {
            config,
            config_path: Mutex::new(None),
            plugin_manager,
            area_manager,
            topic_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            window: Mutex::new(None),
            instance_id,
            instance_type,
            coordinated_size: Mutex::new(None),
            device_metadata: Mutex::new(None),
            button_map: Mutex::new(None),
            close_handler_id: Mutex::new(None),
            lifecycle: Mutex::new(LauncherInstanceLifecycle::Ready),
            auto_stop_handle: Mutex::new(None),
        }
    }

    /// Returns `true` if the instance is in the `Running` lifecycle state.
    pub fn is_running(&self) -> bool {
        self.lifecycle.lock().map(|g| *g == LauncherInstanceLifecycle::Running).unwrap_or(false)
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

    /// Set up areas for a headless instance (no GTK window).
    ///
    /// Creates a main container and adds areas from config so that the
    /// area manager tracks visible areas and their plugins. This is needed
    /// for MacroPad instances where button rendering and input routing
    /// depend on the area manager's state.
    pub fn build_headless(&self) {
        let main_container = HeadlessContainer;

        if let Ok(area_manager) = self.area_manager.lock() {
            area_manager.remove_all_areas_keep_plugins();
            if let Err(e) = area_manager.set_main_container_headless(main_container) {
                error!("Failed to set main container for headless instance '{}': {}", self.instance_id, e);
            }
        }

        let monitor_index = self.config.launcher.layer.monitor;
        let (areas, entries) = self.config.get_layout_for_context(None, monitor_index, None);

        for area_id in areas {
            if let Some(ConfigEntry::Area(area_config)) = entries.get(area_id) {
                if let Ok(area_manager) = self.area_manager.lock() {
                    if let Err(e) = area_manager.add_area_from_config(area_id, area_config.clone()) {
                        error!("Failed to add area {} to headless instance '{}': {}", area_id, self.instance_id, e);
                    } else {
                        debug!("Added area {} to headless instance '{}'", area_id, self.instance_id);
                    }
                }
            }
        }
    }

    pub fn build_window(&self, app: &Application) -> miette::Result<ApplicationWindow> {
        let config = &self.config;
        let launcher_config = config.launcher.clone();
        let rotation = config.launcher.rotation.clone();
        let layout_config = config.layout.clone();

        validate_monitor_index(launcher_config.layer.monitor, &self.instance_id);

        let coordinated_size = self.coordinated_size.lock().ok().and_then(|g| *g);
        let window = create_window(app, &launcher_config, coordinated_size);
        window.add_css_class(&format!("instance-{}", sanitize_css_class_name(&self.instance_id)));
        for css_class in &launcher_config.css_classes {
            window.add_css_class(css_class);
        }

        apply_global_scaled_css(smearor_swipe_launcher_plugin_api::sanitize_scale(launcher_config.scale));

        let app_clone = app.clone();
        let handler_id = window.connect_close_request(move |_win| {
            let app = app_clone.clone();
            gtk4::glib::idle_add_local_once(move || {
                app.quit();
            });
            gtk4::glib::Propagation::Proceed
        });
        if let Ok(mut guard) = self.close_handler_id.lock() {
            *guard = Some(handler_id);
        }

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
            area_manager.remove_all_areas_keep_plugins();
            if let Err(e) = area_manager.set_main_container_gtk(main_container) {
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
    /// Removes all existing area overlays (keeping plugins loaded) and adds
    /// new ones from the given layout. Any plugins required by the new layout
    /// that are not yet loaded are loaded before adding the areas.
    /// Used when the layout profile changes due to monitor hotplug or
    /// workspace changes.
    pub fn rebuild_areas(&self, areas: &[String], entries: &HashMap<String, ConfigEntry>) {
        if let Ok(area_manager) = self.area_manager.lock() {
            area_manager.remove_all_areas_keep_plugins();

            for area_id in areas {
                let area_config = entries.get(area_id).or_else(|| self.config.entries.get(area_id)).and_then(|entry| match entry {
                    ConfigEntry::Area(config) => Some(config.clone()),
                    _ => None,
                });

                if let Some(area_config) = area_config {
                    for plugin_entry in &area_config.plugins {
                        if plugin_entry.disabled {
                            continue;
                        }
                        let namespaced_id = self.plugin_manager.namespaced_plugin_id(&plugin_entry.id);
                        if !self.plugin_manager.plugins.contains_key(&namespaced_id) {
                            debug!("Loading plugin {} for area {}", plugin_entry.id, area_id);
                            let plugin_config = self.config.plugin_config(&plugin_entry.id);
                            if let Err(error) = self.plugin_manager.load_plugin(plugin_entry, plugin_config) {
                                error!("Failed to load plugin {} for area {}: {}", plugin_entry.id, area_id, error);
                            }
                        }
                    }

                    if let Err(error) = area_manager.add_area_from_config(area_id, area_config) {
                        error!("Failed to add area {area_id}: {error}");
                    } else {
                        debug!("Successfully rebuilt area {area_id}");
                    }
                } else {
                    error!("Area config not found for '{area_id}' in profile or top-level entries");
                }
            }
        }
    }

    /// Handle a workspace change event from a compositor service.
    ///
    /// Re-evaluates the layout profile with the new workspace ID and monitor
    /// index, then rebuilds areas only if the resolved layout differs from
    /// the current one.
    pub fn on_workspace_changed(&self, workspace_id: i32, monitor_index: u32) {
        if !self.is_running() {
            debug!(
                "Instance {} skipping workspace change to {} on monitor {} — not running",
                self.instance_id, workspace_id, monitor_index
            );
            return;
        }

        let (areas, entries) = self.config.get_layout_for_context(None, Some(monitor_index), Some(workspace_id));

        let mut current_area_ids: Vec<String> = if let Ok(area_manager) = self.area_manager.lock() {
            area_manager.list_areas().into_iter().map(|a| a.area_id).collect()
        } else {
            Vec::new()
        };
        current_area_ids.sort();

        let mut new_area_ids: Vec<String> = areas.clone();
        new_area_ids.sort();

        if current_area_ids == new_area_ids {
            debug!(
                "Instance {} workspace changed to {} on monitor {}, layout unchanged — skipping rebuild",
                self.instance_id, workspace_id, monitor_index
            );
            return;
        }

        debug!(
            "Instance {} re-evaluating layout for workspace {} on monitor {} (was {:?}, now {:?})",
            self.instance_id, workspace_id, monitor_index, current_area_ids, new_area_ids
        );
        self.rebuild_areas(areas, entries);
    }

    /// Handle a monitor hotplug event from a compositor service.
    ///
    /// Re-evaluates the monitor mapping and rebuilds areas if the monitor
    /// configuration affects this instance.
    pub fn on_monitor_changed(&self, monitor_index: u32, connector_name: &str, change_type: MonitorChangeType) {
        debug!("Instance {} monitor {} ({}): {:?}", self.instance_id, monitor_index, connector_name, change_type);
        if !self.is_running() {
            debug!("Instance {} skipping monitor change — not running", self.instance_id);
            return;
        }
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
