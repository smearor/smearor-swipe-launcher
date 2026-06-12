use crate::SwipeLauncherConfig;
use crate::area::area_stack::AreaStack;
use crate::area::layout_transition::LayoutTransition;
use crate::area::managed_area::ManagedArea;
use crate::config::area::area_type::AreaType;
use crate::config::area::config::AreaConfig;
use crate::config::area::transition::AreaTransition;
use crate::plugin_manager::PluginManager;
use gtk4::Box as GtkBox;
use gtk4::EventControllerKey;
use gtk4::GestureClick;
use gtk4::Orientation;
use gtk4::PolicyType;
use gtk4::ScrolledWindow;
use gtk4::Widget;
use gtk4::gdk::Key;
use gtk4::glib;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;
use tracing::error;
use tracing::info;

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
    pub fn add_area_from_config(&mut self, area_id: &str, area_config: AreaConfig, main_container: &GtkBox) -> Result<(), String> {
        debug!("Adding area {} from config", area_id);

        if self.areas.contains_key(area_id) {
            return Err(format!("Area {} already exists", area_id));
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
    pub fn remove_area(&mut self, area_id: &str, main_container: &GtkBox) -> Result<(), String> {
        debug!("Removing area {}", area_id);

        let managed_area = self.areas.remove(area_id).ok_or_else(|| format!("Area {} not found", area_id))?;

        // Unload plugins for this area
        for plugin_entry in &managed_area.config.plugins {
            self.plugin_manager.unload_plugin(&plugin_entry.id);
        }

        // Animate removal before cleanup
        let widget_clone = managed_area.widget.clone();
        let main_container_clone = main_container.clone();
        let area_id_clone = area_id.to_string();

        self.layout_transition
            .animate_widget_removal(&managed_area.widget, &managed_area.config.transition, move || {
                main_container_clone.remove(&widget_clone);
                debug!("Successfully removed area {}", area_id_clone);
            });

        info!("Successfully initiated removal of area {}", area_id);
        Ok(())
    }

    /// Add a transient area with auto-close detection
    pub fn add_transient_area(&mut self, area_id: &str, area_config: AreaConfig, main_container: &GtkBox) -> Result<(), String> {
        debug!("Adding transient area {}", area_id);

        let mut config = area_config;
        config.auto_close = true;

        self.add_area_from_config(area_id, config, main_container)?;

        // Push to area stack for nested sub-menus
        self.area_stack.push(area_id.to_string());

        if let Some(managed_area) = self.areas.get(area_id) {
            let area_id_clone = area_id.to_string();
            let widget_clone = managed_area.widget.clone();
            let main_container_clone = main_container.clone();
            let close_on_escape = managed_area.config.close_on_escape;

            // Add click-outside detection
            let click_controller = GestureClick::new();
            click_controller.set_button(0);
            click_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

            let area_id_click = area_id_clone.clone();
            let widget_click = widget_clone.clone();
            let _main_container_click = main_container_clone.clone();

            click_controller.connect_pressed(move |gesture, _n_press, x, y| {
                let widget = gesture.widget();
                if let Some(widget) = widget {
                    let _widget_ref = widget.upcast_ref::<Widget>();

                    // Check if click is outside the area widget
                    if !widget_click.contains(x as f64, y as f64) {
                        debug!("Click outside transient area {}", area_id_click);
                        // Remove the area (this is a simplified implementation)
                        // In a real implementation, we'd need to access the AreaManager
                    }
                }
            });

            main_container.add_controller(click_controller);

            // Add escape key handler if configured
            if close_on_escape {
                let key_controller = EventControllerKey::new();
                key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

                let area_id_key = area_id_clone.clone();
                let _widget_key = widget_clone.clone();

                key_controller.connect_key_pressed(move |_, keyval, _, _| {
                    if keyval == Key::Escape {
                        debug!("Escape key pressed for transient area {}", area_id_key);
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                });

                main_container.add_controller(key_controller);
            }
        }

        Ok(())
    }

    /// Pop the top area from the stack and remove it
    pub fn pop_area(&mut self, main_container: &GtkBox) -> Result<Option<String>, String> {
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
    fn create_area_widget(&self, area_config: &AreaConfig) -> Result<Widget, String> {
        match area_config.area_type {
            AreaType::Fixed => {
                let width = area_config.width.unwrap_or(200);
                let mut css_classes = vec!["static-area"];
                self.apply_transition_css_classes(&mut css_classes, &area_config.transition);

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
                self.apply_transition_css_classes(&mut css_classes, &area_config.transition);

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
            if let Some(plugin) = self.plugin_manager.plugins.get(&plugin_entry.id) {
                unsafe {
                    if let Some(ffi_widget) = plugin.build_widget() {
                        let widget = Widget::from_glib_full(ffi_widget.raw_widget);
                        container.append(&widget);
                        info!("Plugin {} successfully added to area widget", plugin_entry.id);
                    } else {
                        error!("Plugin {} failed to build widget", plugin_entry.id);
                    }
                }
            }
        }
    }

    /// Apply CSS classes based on transition type
    fn apply_transition_css_classes(&self, css_classes: &mut Vec<&str>, transition: &AreaTransition) {
        match transition {
            AreaTransition::None => {
                css_classes.push("transition-none");
            }
            AreaTransition::Fade => {
                css_classes.push("transition-fade");
            }
            AreaTransition::SlideLeft => {
                css_classes.push("transition-slide-left");
            }
            AreaTransition::SlideRight => {
                css_classes.push("transition-slide-right");
            }
            AreaTransition::SlideUp => {
                css_classes.push("transition-slide-up");
            }
            AreaTransition::SlideDown => {
                css_classes.push("transition-slide-down");
            }
            AreaTransition::Pop => {
                css_classes.push("transition-pop");
            }
            AreaTransition::Scale => {
                css_classes.push("transition-scale");
            }
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
                glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
                    widget_clone.set_opacity(1.0);
                    glib::ControlFlow::Continue
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
