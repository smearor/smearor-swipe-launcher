use crate::SwipeLauncherConfig;
use crate::area::area_stack::AreaStack;
use crate::area::error::AddAreaError;
use crate::area::error::CreateAreaError;
use crate::area::error::RemoveAreaError;
use crate::area::layout_transition::LayoutTransition;
use crate::area::managed_area::ManagedArea;
use crate::config::area::area_type::AreaType;
use crate::config::area::config::AreaConfig;
use crate::config::area::transition::AreaTransition;
use crate::plugin_manager::PluginManager;
use gtk4::Box as GtkBox;
use gtk4::Orientation;
use gtk4::Overlay;
use gtk4::PolicyType;
use gtk4::ScrolledWindow;
use gtk4::Widget;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Manages dynamic area operations at runtime
pub struct AreaManager {
    /// Currently managed areas keyed by ID
    areas: HashMap<String, ManagedArea>,

    /// Stack of areas for nested sub-menus
    area_stack: AreaStack,

    /// Layout transition animator
    layout_transition: LayoutTransition,

    /// Reference to the plugin manager for loading plugins
    plugin_manager: Arc<PluginManager>,

    /// Reference to the main configuration
    config: Arc<SwipeLauncherConfig>,
}

impl AreaManager {
    /// Create a new AreaManager
    pub fn new(plugin_manager: Arc<PluginManager>, config: Arc<SwipeLauncherConfig>) -> Self {
        Self {
            areas: HashMap::new(),
            area_stack: AreaStack::new(),
            layout_transition: LayoutTransition::new(),
            plugin_manager,
            config,
        }
    }

    /// Add an area from configuration
    pub fn add_area_from_config(&mut self, area_id: &str, area_config: AreaConfig, main_container: &GtkBox) -> Result<(), AddAreaError> {
        debug!("Adding area {} from config", area_id);

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
            .animate_widget_addition(&overlay.clone().upcast::<Widget>(), &area_config.open_transition);

        info!("Successfully added area {} with overlay", area_id);
        Ok(())
    }

    /// Remove an area with plugin cleanup
    pub fn remove_area(&mut self, area_id: &str, main_container: &GtkBox) -> Result<(), RemoveAreaError> {
        debug!("Removing area {}", area_id);

        let managed_area = self.areas.remove(area_id).ok_or_else(|| RemoveAreaError::AreaNotFound(area_id.to_string()))?;

        // Unload plugins for this area
        for plugin_entry in &managed_area.config.plugins {
            self.plugin_manager.unload_plugin(&plugin_entry.id);
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

        let close_transition = managed_area.config.close_transition.unwrap_or(managed_area.config.open_transition.opposite());

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

        info!("Successfully initiated removal of area {}", area_id);
        Ok(())
    }

    /// Add a transient area with auto-close detection
    pub fn add_transient_area(
        &mut self,
        area_id: &str,
        area_config: AreaConfig,
        _main_container: &GtkBox,
        sender_id: Option<&str>,
    ) -> Result<(), AddAreaError> {
        debug!("Adding transient area {} from sender {:?}", area_id, sender_id);

        // Check if area already exists
        if self.areas.contains_key(area_id) {
            return Err(AddAreaError::AreaAlreadyExists(area_id.to_string()));
        }

        let mut config = area_config;
        config.auto_close = true;

        // Load plugins for this transient area
        for plugin_entry in &config.plugins {
            if !self.plugin_manager.plugins.contains_key(&plugin_entry.id) {
                let plugin_config = self.config.plugin_config(&plugin_entry.id);
                info!("Loading plugin {} for transient area {}", plugin_entry.id, area_id);
                if let Err(e) = self.plugin_manager.load_plugin(plugin_entry, plugin_config) {
                    error!("Failed to load plugin {} for transient area {}: {}", plugin_entry.id, area_id, e);
                } else {
                    info!("Successfully loaded plugin {} for transient area {}", plugin_entry.id, area_id);
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
            info!("Made source area widget transparent for {}", area_id);
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
            info!("Added transient area {} overlay to source area overlay", area_id);
        } else {
            // Fallback: if no source overlay found, this shouldn't happen but handle gracefully
            warn!("No source area overlay found for transient area {}", area_id);
        }

        // Push to area stack for nested sub-menus
        self.area_stack.push(area_id.to_string());

        // Apply transition animation
        self.layout_transition.animate_widget_addition(&widget, &config.open_transition);

        info!("Successfully added transient area {}", area_id);
        Ok(())
    }

    /// Find the overlay and widget of the area that contains a specific plugin
    fn find_area_overlay_and_widget_containing_plugin(&self, plugin_id: &str) -> (Option<Overlay>, Option<Widget>) {
        for managed_area in self.areas.values() {
            // Check if this area contains the plugin (including transient areas)
            if managed_area.config.plugins.iter().any(|p| p.id == plugin_id) {
                return (managed_area.overlay.clone(), Some(managed_area.widget.clone()));
            }
        }
        (None, None)
    }

    /// Find the area ID for a given widget
    fn find_area_id_for_widget(&self, widget: &Widget) -> Option<String> {
        for (area_id, managed_area) in &self.areas {
            if managed_area.widget == *widget {
                return Some(area_id.clone());
            }
        }
        None
    }

    /// Find the overlay that has a specific widget as its child
    fn find_overlay_for_widget(&self, widget: &Widget) -> Option<Overlay> {
        for managed_area in self.areas.values() {
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

    /// Pop the top area from the stack and remove it
    pub fn pop_area(&mut self, main_container: &GtkBox) -> Result<Option<String>, RemoveAreaError> {
        if let Some(area_id) = self.area_stack.pop() {
            self.remove_area(&area_id, main_container)?;
            Ok(Some(area_id))
        } else {
            Ok(None)
        }
    }

    /// Get the current area stack
    pub fn get_area_stack(&self) -> &AreaStack {
        &self.area_stack
    }

    /// Create the GTK widget for an area based on its configuration
    fn create_area_widget(&self, area_config: &AreaConfig) -> Result<Widget, CreateAreaError> {
        match area_config.area_type {
            AreaType::Fixed => {
                let width = area_config.width.unwrap_or(200);
                let mut css_classes = vec!["static-area"];
                css_classes.push(area_config.open_transition.css_class());

                let box_widget = GtkBox::builder()
                    .orientation(Orientation::Horizontal)
                    .width_request(width)
                    .css_classes(css_classes.as_slice())
                    .build();
                self.add_plugins(area_config, &box_widget);

                self.apply_transition_animation(box_widget.upcast_ref(), &area_config.open_transition);

                Ok(box_widget.upcast())
            }
            AreaType::Scroll => {
                let mut css_classes = vec!["scroll-area"];
                css_classes.push(area_config.open_transition.css_class());

                let scrolled_window = ScrolledWindow::builder()
                    .hscrollbar_policy(PolicyType::External)
                    .vscrollbar_policy(PolicyType::Never)
                    .hexpand(true)
                    .vexpand(true)
                    .css_classes(css_classes.as_slice())
                    .build();

                let plugin_container = GtkBox::builder().orientation(Orientation::Horizontal).spacing(10).build();
                self.add_plugins(area_config, &plugin_container);

                // TODO: Apply drag gesture for scrolling

                self.apply_transition_animation(scrolled_window.upcast_ref(), &area_config.open_transition);

                scrolled_window.set_child(Some(&plugin_container));
                Ok(scrolled_window.upcast())
            }
        }
    }

    fn add_plugins(&self, area_config: &AreaConfig, container: &GtkBox) {
        for plugin_entry in &area_config.plugins {
            let Some(plugin) = self.plugin_manager.plugins.get(&plugin_entry.id) else {
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
            info!("Plugin {} successfully added to area widget", plugin_entry.id);
        }
    }

    /// Apply transition animation to a widget
    fn apply_transition_animation(&self, widget: &Widget, transition: &AreaTransition) {
        self.layout_transition.animate_widget_addition(widget, transition);
    }

    /// Get a managed area by ID
    pub fn get_area(&self, area_id: &str) -> Option<&ManagedArea> {
        self.areas.get(area_id)
    }

    /// Get all managed areas
    pub fn get_all_areas(&self) -> &HashMap<String, ManagedArea> {
        &self.areas
    }

    /// Check if an area exists
    pub fn has_area(&self, area_id: &str) -> bool {
        self.areas.contains_key(area_id)
    }
}
