use crate::SwipeLauncherConfig;
// use crate::area::area_stack::AreaStack;
use crate::area::error::AddAreaError;
use crate::area::error::CreateAreaError;
use crate::area::error::MainContainerInitializationError;
use crate::area::error::MainContainerNotInitialized;
use crate::area::error::RemoveAreaError;
use crate::area::layout_transition::LayoutTransition;
use crate::area::managed_area::ManagedArea;
use crate::plugin_manager::PluginManager;
use dashmap::DashMap;
use gtk4::Align;
use gtk4::Box as GtkBox;
use gtk4::Orientation;
use gtk4::Overlay;
use gtk4::PolicyType;
use gtk4::ScrolledWindow;
use gtk4::Widget;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use smearor_model_area::AreaAlign;
use smearor_model_area::AreaConfig;
use smearor_model_area::AreaType;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::trace;
use tracing::warn;

/// Snapshot of an area's current state for external consumers such as the
/// MCP server.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AreaInfo {
    pub area_id: String,
    pub visible: bool,
    pub focused: bool,
    pub position: String,
    pub active: bool,
}

/// Information about all configured areas, including those not yet opened.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AllAreaInfo {
    pub area_id: String,
    pub visible: bool,
    pub active: bool,
    pub area_type: String,
}

/// Manages dynamic area operations at runtime
pub struct AreaManager {
    /// Currently managed areas keyed by ID
    areas: DashMap<String, ManagedArea>,

    // /// Stack of areas for nested sub-menus
    // area_stack: AreaStack,
    /// Layout transition animator
    layout_transition: LayoutTransition,

    /// Reference to the plugin manager for loading plugins
    plugin_manager: Arc<PluginManager>,

    /// Reference to the main configuration
    pub(crate) config: Arc<SwipeLauncherConfig>,

    /// Reference to the main container
    pub(crate) main_container: Arc<RwLock<Option<GtkBox>>>,
}

impl AreaManager {
    /// Create a new AreaManager
    pub fn new(plugin_manager: Arc<PluginManager>, config: Arc<SwipeLauncherConfig>, json_converter_registry: Arc<JsonConverterRegistry>) -> Self {
        smearor_model_area::register_json_converters_in_registry(&json_converter_registry);

        Self {
            areas: DashMap::new(),
            // area_stack: AreaStack::new(),
            layout_transition: LayoutTransition::new(),
            plugin_manager,
            config,
            main_container: Arc::new(RwLock::new(None)),
        }
    }

    /// Add an area from configuration
    pub fn add_area_from_config(&self, area_id: &str, area_config: AreaConfig) -> Result<(), AddAreaError> {
        trace!("Adding area {} from config", area_id);

        let main_container = self.get_main_container()?;

        if self.areas.contains_key(area_id) {
            return Err(AddAreaError::AreaAlreadyExists(area_id.to_string()));
        }

        let widget = self.create_area_widget(&area_config)?;

        // Create overlay for this area (for transient sub-areas)
        let overlay = Overlay::builder().build();
        overlay.set_child(Some(&widget.clone()));
        overlay.add_css_class("area-overlay");

        let managed_area = ManagedArea {
            id: area_id.to_string(),
            config: area_config.clone(),
            widget: widget.clone(),
            overlay: Some(overlay.clone()),
            source_area_widget: None,
            is_transient: area_config.auto_close,
        };

        self.areas.insert(area_id.to_string(), managed_area);
        main_container.append(&overlay);

        // Apply transition animation
        self.layout_transition
            .animate_widget_addition(&overlay.clone().upcast::<Widget>(), &area_config.open_transition());
        // self.layout_transition.animate_widget_addition(&widget, &area_config.close_transition());

        trace!("Successfully added area {} with overlay", area_id);
        Ok(())
    }

    /// Remove an area with plugin cleanup
    pub fn remove_area(&self, area_id: &str) -> Result<(), RemoveAreaError> {
        debug!("Removing area {}", area_id);

        let main_container = self.get_main_container()?;

        let (area_id, managed_area) = self.areas.remove(area_id).ok_or_else(|| RemoveAreaError::AreaNotFound(area_id.to_string()))?;

        // Unload plugins for this area
        for plugin_entry in &managed_area.config.plugins {
            let namespaced_id = self.plugin_manager.namespaced_plugin_id(&plugin_entry.id);
            self.plugin_manager.unload_plugin(&namespaced_id);
        }

        // Animate removal before cleanup
        let _widget_clone = managed_area.widget.clone();
        let overlay_clone = managed_area.overlay.clone();
        let main_container_clone = main_container.clone();
        let area_id_clone = area_id.to_string();
        let is_transient = managed_area.is_transient;
        let source_area_widget_clone = managed_area.source_area_widget.clone();
        let source_area_overlay_clone = if is_transient {
            // Find the source area overlay for this transient area
            if let Some(ref source_widget) = managed_area.source_area_widget {
                self.find_overlay_for_widget(source_widget)
            } else {
                None
            }
        } else {
            None
        };

        let close_transition = managed_area.config.close_transition();

        self.layout_transition.animate_widget_removal(&managed_area.widget, &close_transition, move || {
            if is_transient {
                // Transient area: remove overlay from source area overlay
                if let Some(overlay) = &overlay_clone {
                    if let Some(source_overlay) = &source_area_overlay_clone {
                        source_overlay.remove_overlay(overlay);
                    }
                }
                // Restore source area widget visibility
                if let Some(ref source_widget) = source_area_widget_clone {
                    source_widget.remove_css_class("scroll-area-transparent");
                    debug!("Restored source area widget visibility for {}", area_id_clone);
                }
            } else {
                // Non-transient area: remove the overlay from main container
                if let Some(overlay) = &overlay_clone {
                    main_container_clone.remove(overlay);
                }
            }
            debug!("Successfully removed area {}", area_id_clone);
        });

        trace!("Successfully initiated removal of area {}", area_id);
        Ok(())
    }

    /// Remove all areas synchronously without animation.
    ///
    /// Called during shutdown to ensure no pending GLib timeout callbacks
    /// (from `animate_widget_removal`) remain active when services are
    /// unloaded and the process exits.
    pub fn remove_all_areas_immediate(&self) {
        debug!("Removing all areas immediately (shutdown)");

        let main_container = match self.get_main_container() {
            Ok(container) => container,
            Err(_) => {
                debug!("Main container not initialized, skipping area removal");
                return;
            }
        };

        let area_ids: Vec<String> = self.areas.iter().map(|entry| entry.key().to_string()).collect();
        for area_id in &area_ids {
            if let Some((_, managed_area)) = self.areas.remove(area_id) {
                for plugin_entry in &managed_area.config.plugins {
                    let namespaced_id = self.plugin_manager.namespaced_plugin_id(&plugin_entry.id);
                    self.plugin_manager.unload_plugin(&namespaced_id);
                }

                if let Some(overlay) = &managed_area.overlay {
                    main_container.remove(overlay);
                }
                debug!("Removed area {} immediately", area_id);
            }
        }
    }

    /// Remove all areas immediately without animation, unloading plugins.
    ///
    /// Used for runtime layout switching when the layout profile changes
    /// (e.g. on monitor hotplug). After clearing, new areas can be added
    /// via `add_area_from_config`.
    pub fn clear_areas(&self) {
        self.remove_all_areas_immediate();
    }

    /// Add a transient area with auto-close detection
    pub fn add_transient_area(&self, area_id: &str, area_config: AreaConfig, sender_id: Option<&str>) -> Result<(), AddAreaError> {
        trace!("Adding transient area {} from sender {:?}", area_id, sender_id);

        // Check if area already exists
        if self.areas.contains_key(area_id) {
            return Err(AddAreaError::AreaAlreadyExists(area_id.to_string()));
        }

        let mut config = area_config;
        config.auto_close = true;

        // Load plugins for this transient area
        for plugin_entry in &config.plugins {
            if plugin_entry.disabled {
                trace!("Skipping disabled plugin {} for transient area {}", plugin_entry.id, area_id);
                continue;
            }
            let namespaced_id = self.plugin_manager.namespaced_plugin_id(&plugin_entry.id);
            if !self.plugin_manager.plugins.contains_key(&namespaced_id) {
                let plugin_config = self.config.plugin_config(&plugin_entry.id);
                trace!("Loading plugin {} for transient area {}", plugin_entry.id, area_id);
                if let Err(e) = self.plugin_manager.load_plugin(plugin_entry, plugin_config) {
                    error!("Failed to load plugin {} for transient area {}: {}", plugin_entry.id, area_id, e);
                } else {
                    trace!("Successfully loaded plugin {} for transient area {}", plugin_entry.id, area_id);
                }
            }
        }

        // Find the source area that contains the sender plugin
        let (source_area_overlay, source_area_widget) = if let Some(sender) = sender_id {
            self.find_area_overlay_and_widget_containing_plugin(sender)
        } else {
            (None, None)
        };

        // Make source area widget transparent (overlay remains visible)
        if let Some(ref widget) = source_area_widget {
            widget.add_css_class("scroll-area-transparent");
            trace!("Made source area widget transparent for {}", area_id);
        }

        // Create the area widget
        let widget = self.create_area_widget(&config)?;

        // Create overlay for this transient area (for sub-transient areas)
        let overlay = Overlay::builder().build();
        overlay.set_child(Some(&widget.clone()));
        overlay.add_css_class("area-overlay");

        let managed_area = ManagedArea {
            id: area_id.to_string(),
            config: config.clone(),
            widget: widget.clone(),
            overlay: Some(overlay.clone()),
            source_area_widget: source_area_widget.clone(),
            is_transient: true,
        };

        self.areas.insert(area_id.to_string(), managed_area);

        // Add overlay to source area overlay (not the widget)
        if let Some(source_overlay) = source_area_overlay {
            source_overlay.add_overlay(&overlay);
            trace!("Added transient area {} overlay to source area overlay", area_id);
        } else {
            // Fallback: if no source overlay found, this shouldn't happen but handle gracefully
            warn!("No source area overlay found for transient area {}", area_id);
        }

        // // Push to area stack for nested sub-menus
        // self.area_stack.push(area_id.to_string());

        // Apply transition animation
        self.layout_transition.animate_widget_addition(&widget, &config.open_transition());
        // let overlay_widget = overlay.clone().upcast::<Widget>();
        // self.layout_transition.animate_widget_addition(&overlay_widget, &config.close_transition());

        trace!("Successfully added transient area {}", area_id);
        Ok(())
    }

    /// Find the overlay and widget of the area that contains a specific plugin.
    /// `plugin_id` may be namespaced (`instance_id:plugin_id`); the raw ID is extracted.
    fn find_area_overlay_and_widget_containing_plugin(&self, plugin_id: &str) -> (Option<Overlay>, Option<Widget>) {
        let raw_plugin_id = plugin_id.rsplit_once(':').map(|(_, id)| id).unwrap_or(plugin_id);
        for managed_area in &self.areas {
            // Check if this area contains the plugin (including transient areas)
            if managed_area.config.plugins.iter().any(|p| p.id == raw_plugin_id) {
                return (managed_area.overlay.clone(), Some(managed_area.widget.clone()));
            }
        }
        (None, None)
    }

    /// Find the area ID for a given widget
    fn find_area_id_for_widget(&self, widget: &Widget) -> Option<String> {
        for managed_area in &self.areas {
            if managed_area.value().widget == *widget {
                return Some(managed_area.key().clone());
            }
        }
        None
    }

    /// Find the overlay that has a specific widget as its child
    fn find_overlay_for_widget(&self, widget: &Widget) -> Option<Overlay> {
        for managed_area in &self.areas {
            if let Some(ref overlay) = managed_area.overlay {
                if let Some(child) = overlay.child() {
                    if child == *widget {
                        return Some(overlay.clone());
                    }
                }
            }
        }
        None
    }

    // /// Pop the top area from the stack and remove it
    // pub fn pop_area(&mut self) -> Result<Option<String>, RemoveAreaError> {
    //     if let Some(area_id) = self.area_stack.pop() {
    //         self.remove_area(&area_id)?;
    //         Ok(Some(area_id))
    //     } else {
    //         Ok(None)
    //     }
    // }

    // /// Get the current area stack
    // pub fn get_area_stack(&self) -> &AreaStack {
    //     &self.area_stack
    // }
    //
    /// Create the GTK widget for an area based on its configuration
    fn create_area_widget(&self, area_config: &AreaConfig) -> Result<Widget, CreateAreaError> {
        match area_config.area_type {
            AreaType::Fixed => {
                let width = area_config.width.unwrap_or(200);
                let mut css_classes = vec!["static-area"];
                css_classes.push(area_config.open_transition.css_class());
                css_classes.extend(area_config.css_classes.iter().map(String::as_str));

                let box_widget = GtkBox::builder()
                    .orientation(Orientation::Horizontal)
                    .width_request(width)
                    .css_classes(css_classes.as_slice())
                    .build();

                let inner = GtkBox::builder()
                    .orientation(Orientation::Horizontal)
                    .spacing(area_config.spacing)
                    .halign(match area_config.effective_align() {
                        AreaAlign::Left => Align::Start,
                        AreaAlign::Center => Align::Center,
                        AreaAlign::Right => Align::End,
                    })
                    .build();
                box_widget.append(&inner);
                self.add_plugins(area_config, &inner);

                // self.apply_transition_animation(box_widget.upcast_ref(), &area_config.open_transition);

                Ok(box_widget.upcast())
            }
            AreaType::Scroll => {
                let mut css_classes = vec!["scroll-area"];
                css_classes.push(area_config.open_transition.css_class());
                css_classes.extend(area_config.css_classes.iter().map(String::as_str));

                let scrolled_window = ScrolledWindow::builder()
                    .hscrollbar_policy(PolicyType::External)
                    .vscrollbar_policy(PolicyType::Never)
                    .hexpand(true)
                    .vexpand(true)
                    .css_classes(css_classes.as_slice())
                    .build();

                let plugin_container = GtkBox::builder().orientation(Orientation::Horizontal).spacing(area_config.spacing).build();
                for class in &area_config.css_classes {
                    plugin_container.add_css_class(class);
                }
                match area_config.effective_align() {
                    AreaAlign::Left => {
                        self.add_plugins(area_config, &plugin_container);
                    }
                    AreaAlign::Center => {
                        let left_spacer = GtkBox::builder().hexpand(true).build();
                        plugin_container.append(&left_spacer);
                        self.add_plugins(area_config, &plugin_container);
                        let right_spacer = GtkBox::builder().hexpand(true).build();
                        plugin_container.append(&right_spacer);
                    }
                    AreaAlign::Right => {
                        let left_spacer = GtkBox::builder().hexpand(true).build();
                        plugin_container.append(&left_spacer);
                        self.add_plugins(area_config, &plugin_container);
                    }
                }

                // TODO: Apply drag gesture for scrolling

                // self.apply_transition_animation(scrolled_window.upcast_ref(), &area_config.open_transition);

                scrolled_window.set_child(Some(&plugin_container));
                Ok(scrolled_window.upcast())
            }
        }
    }

    fn add_plugins(&self, area_config: &AreaConfig, container: &GtkBox) {
        for plugin_entry in &area_config.plugins {
            if plugin_entry.disabled {
                continue;
            }
            let namespaced_id = self.plugin_manager.namespaced_plugin_id(&plugin_entry.id);
            let Some(plugin) = self.plugin_manager.plugins.get(&namespaced_id) else {
                continue;
            };
            let widget = unsafe {
                let Some(ffi_widget) = plugin.build_widget() else {
                    error!("Plugin {} failed to build widget", plugin_entry.id);
                    continue;
                };
                Widget::from_glib_full(ffi_widget.raw_widget)
            };
            container.append(&widget);
            trace!("Plugin {} successfully added to area widget", plugin_entry.id);
        }
    }

    // /// Apply transition animation to a widget
    // fn apply_transition_animation(&self, widget: &Widget, transition: &AreaTransition) {
    //     self.layout_transition.animate_widget_addition(widget, transition);
    // }

    // /// Get a managed area by ID
    // pub fn get_area(&self, area_id: &str) -> Option<&ManagedArea> {
    //     self.areas.get(area_id)
    //     // self.areas.get(area_id)
    // }

    // /// Get all managed areas
    // pub fn get_all_areas(&self) -> &HashMap<String, ManagedArea> {
    //     &self.areas
    // }

    pub fn get_first_scrolled_window(&self, area_id: &str) -> Option<ScrolledWindow> {
        let Some(managed_area) = self.areas.get(area_id) else {
            return None;
        };
        if managed_area.config.area_type != AreaType::Scroll {
            return None;
        }
        let Some(scrolled_window) = managed_area.widget.downcast_ref::<ScrolledWindow>() else {
            return None;
        };
        Some(scrolled_window.clone())
    }

    /// Check if an area exists
    pub fn has_area(&self, area_id: &str) -> bool {
        self.areas.contains_key(area_id)
    }

    /// Find the area that contains a button whose click_payload references
    /// the given area_id. Returns the plugin ID of that button to use as
    /// sender_id for transient area opening.
    pub fn find_area_containing_area_button(&self, target_area_id: &str) -> Option<String> {
        for managed_area in &self.areas {
            for plugin_entry in &managed_area.config.plugins {
                if let Some(plugin_config) = self.config.get_plugin_config(&plugin_entry.id) {
                    if let Some(click_payload) = plugin_config.get("click_payload") {
                        if click_payload.get("area_id").and_then(|v| v.as_str()) == Some(target_area_id) {
                            return Some(plugin_entry.id.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// Find a plugin ID to use as sender for transient area opening.
    /// If source_area_id is provided, uses the first plugin of that area.
    /// Otherwise, falls back to the first plugin of the first scroll area.
    pub fn find_sender_id_for_transient(&self, source_area_id: Option<&str>) -> Option<String> {
        if let Some(source_id) = source_area_id {
            self.areas
                .get(source_id)
                .and_then(|source_area| source_area.config.plugins.first().map(|p| p.id.clone()))
        } else {
            self.areas
                .iter()
                .find(|a| a.config.area_type == AreaType::Scroll)
                .and_then(|a| a.config.plugins.first().map(|p| p.id.clone()))
        }
    }

    /// Show the area widget. If the area is configured but not yet loaded,
    /// it will be added from config first.
    pub fn ensure_area(&self, area_id: &str) -> Result<(), String> {
        if self.areas.contains_key(area_id) {
            let area = self.areas.get(area_id).unwrap();
            area.widget.set_visible(true);
            debug!("Opened existing area {}", area_id);
            return Ok(());
        }
        let area_config = self
            .config
            .get_area_config(area_id)
            .ok_or_else(|| format!("Area {} not found in config", area_id))?
            .clone();

        // Load plugins for this area (matching add_transient_area pattern)
        for plugin_entry in &area_config.plugins {
            if plugin_entry.disabled {
                debug!("Skipping disabled plugin {} for area {}", plugin_entry.id, area_id);
                continue;
            }
            let namespaced_id = self.plugin_manager.namespaced_plugin_id(&plugin_entry.id);
            if !self.plugin_manager.plugins.contains_key(&namespaced_id) {
                let plugin_config = self.config.plugin_config(&plugin_entry.id);
                debug!("Loading plugin {} for area {}", plugin_entry.id, area_id);
                if let Err(e) = self.plugin_manager.load_plugin(plugin_entry, plugin_config) {
                    error!("Failed to load plugin {} for area {}: {}", plugin_entry.id, area_id, e);
                }
            }
        }

        self.add_area_from_config(area_id, area_config)
            .map_err(|e| format!("Failed to add area {}: {}", area_id, e))?;
        debug!("Added and opened area {} from config", area_id);
        Ok(())
    }

    /// Show the area widget.
    pub fn open(&self, area_id: &str) -> Result<(), String> {
        let Some(area) = self.areas.get(area_id) else {
            return Err(format!("Area {} not found", area_id));
        };
        area.widget.set_visible(true);
        debug!("Opened area {}", area_id);
        Ok(())
    }

    /// Hide the area widget.
    pub fn close(&self, area_id: &str) -> Result<(), String> {
        let Some(area) = self.areas.get(area_id) else {
            return Err(format!("Area {} not found", area_id));
        };
        area.widget.set_visible(false);
        debug!("Closed area {}", area_id);
        Ok(())
    }

    /// Move keyboard focus to the area widget.
    pub fn focus(&self, area_id: &str) -> Result<(), String> {
        let Some(area) = self.areas.get(area_id) else {
            return Err(format!("Area {} not found", area_id));
        };
        area.widget.grab_focus();
        debug!("Focused area {}", area_id);
        Ok(())
    }

    /// List all managed areas with their current state.
    pub fn list_areas(&self) -> Vec<AreaInfo> {
        self.areas
            .iter()
            .map(|area| AreaInfo {
                area_id: area.key().clone(),
                visible: area.widget.is_visible(),
                focused: area.widget.has_focus(),
                position: format!("{:?}", area.config.effective_align()),
                active: true,
            })
            .collect()
    }

    /// List all configured areas (including not-yet-opened ones) with their state.
    /// Iterates over all config entries to find every area definition, not just
    /// the ones listed in the top-level `areas` array.
    pub fn list_all_areas(&self) -> Vec<AllAreaInfo> {
        let mut result: Vec<AllAreaInfo> = self
            .config
            .entries
            .iter()
            .filter_map(|(area_id, entry)| match entry {
                crate::config::area::config_entry::ConfigEntry::Area(area_config) => {
                    let managed = self.areas.get(area_id);
                    Some(AllAreaInfo {
                        area_id: area_id.clone(),
                        visible: managed.as_ref().is_some_and(|a| a.widget.is_visible()),
                        active: managed.is_some(),
                        area_type: format!("{:?}", area_config.area_type),
                    })
                }
                crate::config::area::config_entry::ConfigEntry::Plugin(_) => None,
            })
            .collect();
        result.sort_by(|a, b| a.area_id.cmp(&b.area_id));
        result
    }

    /// Toggle the visibility of an area.
    pub fn toggle(&self, area_id: &str) -> Result<(), String> {
        let Some(area) = self.areas.get(area_id) else {
            return Err(format!("Area {} not found", area_id));
        };
        let visible = area.widget.is_visible();
        area.widget.set_visible(!visible);
        debug!("Toggled area {} to visible={}", area_id, !visible);
        Ok(())
    }

    /// Return the configuration of a managed area.
    pub fn get_area_config(&self, area_id: &str) -> Result<AreaConfig, String> {
        let Some(area) = self.areas.get(area_id) else {
            return Err(format!("Area {} not found", area_id));
        };
        Ok(area.config.clone())
    }

    pub fn set_main_container(&self, main_container: gtk4::Box) -> Result<(), MainContainerInitializationError> {
        let Ok(mut guard) = self.main_container.write() else {
            return Err(MainContainerInitializationError);
        };
        guard.replace(main_container);
        Ok(())
    }

    fn get_main_container(&self) -> Result<gtk4::Box, MainContainerNotInitialized> {
        let Ok(guard) = self.main_container.read() else {
            return Err(MainContainerNotInitialized);
        };
        guard.clone().ok_or(MainContainerNotInitialized)
    }
}
