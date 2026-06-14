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
use gtk4::glib;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::info;

/// Manages dynamic area operations at runtime
pub struct AreaManager {
    /// Currently managed areas keyed by ID
    areas: HashMap<String, ManagedArea>,

    /// Overlay container for transient areas (submenus) - created on demand
    overlay: Option<Overlay>,

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
            overlay: None,
            area_stack: AreaStack::new(),
            layout_transition: LayoutTransition::new(),
            plugin_manager,
            config,
        }
    }

    /// Set the overlay for transient areas (called from application.rs)
    pub fn set_overlay(&mut self, overlay: Overlay) {
        self.overlay = Some(overlay);
    }

    /// Get the overlay for transient areas
    fn get_overlay(&self) -> Option<&Overlay> {
        self.overlay.as_ref()
    }

    /// Add an area from configuration
    pub fn add_area_from_config(&mut self, area_id: &str, area_config: AreaConfig, main_container: &GtkBox) -> Result<(), AddAreaError> {
        debug!("Adding area {} from config", area_id);

        if self.areas.contains_key(area_id) {
            return Err(AddAreaError::AreaAlreadyExists(area_id.to_string()));
        }

        let widget = self.create_area_widget(&area_config)?;

        let managed_area = ManagedArea {
            id: area_id.to_string(),
            config: area_config.clone(),
            widget: widget.clone(),
            is_transient: area_config.auto_close,
        };

        self.areas.insert(area_id.to_string(), managed_area);
        main_container.append(&widget);

        // Apply transition animation
        self.layout_transition.animate_widget_addition(&widget, &area_config.transition);

        info!("Successfully added area {}", area_id);
        Ok(())
    }

    /// Remove an area with plugin cleanup
    pub fn remove_area(&mut self, area_id: &str, _main_container: &GtkBox) -> Result<(), RemoveAreaError> {
        debug!("Removing area {}", area_id);

        let managed_area = self.areas.remove(area_id).ok_or_else(|| RemoveAreaError::AreaNotFound(area_id.to_string()))?;

        // Unload plugins for this area
        for plugin_entry in &managed_area.config.plugins {
            self.plugin_manager.unload_plugin(&plugin_entry.id);
        }

        // Animate removal before cleanup
        let widget_clone = managed_area.widget.clone();
        let overlay_clone = self.overlay.clone();
        let area_id_clone = area_id.to_string();

        self.layout_transition
            .animate_widget_removal(&managed_area.widget, &managed_area.config.transition, move || {
                if let Some(overlay) = &overlay_clone {
                    overlay.remove_overlay(&widget_clone);
                }
                debug!("Successfully removed area {}", area_id_clone);
            });

        info!("Successfully initiated removal of area {}", area_id);
        Ok(())
    }

    /// Add a transient area with auto-close detection
    pub fn add_transient_area(&mut self, area_id: &str, area_config: AreaConfig, _main_container: &GtkBox) -> Result<(), AddAreaError> {
        debug!("Adding transient area {}", area_id);

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

        // Get overlay
        let overlay = self.get_overlay().ok_or_else(|| AddAreaError::OverlayNotSetError)?;
        let overlay_clone = overlay.clone();

        // Create the area widget
        let widget = self.create_area_widget(&config)?;

        let managed_area = ManagedArea {
            id: area_id.to_string(),
            config: config.clone(),
            widget: widget.clone(),
            is_transient: true,
        };

        self.areas.insert(area_id.to_string(), managed_area);

        // Add to overlay
        overlay_clone.add_overlay(&widget);

        // Push to area stack for nested sub-menus
        self.area_stack.push(area_id.to_string());

        // Apply transition animation
        self.layout_transition.animate_widget_addition(&widget, &config.transition);

        info!("Successfully added transient area {}", area_id);
        Ok(())
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
                css_classes.push(area_config.transition.css_class());

                let box_widget = GtkBox::builder()
                    .orientation(Orientation::Horizontal)
                    .width_request(width)
                    .css_classes(css_classes.as_slice())
                    .build();
                self.add_plugins(area_config, &box_widget);

                self.apply_transition_animation(box_widget.upcast_ref(), &area_config.transition);

                Ok(box_widget.upcast())
            }
            AreaType::Scroll => {
                let mut css_classes = vec!["scroll-area"];
                css_classes.push(area_config.transition.css_class());

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

                self.apply_transition_animation(scrolled_window.upcast_ref(), &area_config.transition);

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
        match transition {
            AreaTransition::None => {
                // No animation
            }
            AreaTransition::Fade => {
                widget.set_opacity(0.0);
                let widget_clone = widget.clone();
                glib::timeout_add_local(Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    return glib::ControlFlow::Continue;
                });
            }
            _ => {
                // For other transitions, we'll rely on CSS animations
                debug!("Applying transition animation: {:?}", transition);
            }
        }
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
