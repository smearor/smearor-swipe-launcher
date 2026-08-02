use crate::SwipeLauncherConfig;
use crate::area::area_info::AllAreaInfo;
use crate::area::area_info::AreaInfo;
use crate::area::area_source::AreaSource;
use crate::area::backend::AreaBackend;
use crate::area::backend::HeadlessBackend;
use crate::area::container::AreaContainer;
use crate::area::container::GtkBackend;
use crate::area::error::AddAreaError;
use crate::area::error::MainContainerInitializationError;
use crate::area::error::MainContainerNotInitialized;
use crate::area::error::RemoveAreaError;
use crate::area::managed_area::ManagedArea;
use crate::area::overlay::AreaOverlay;
use crate::area::widget::AreaWidget;
use crate::plugin_manager::PluginManager;
use dashmap::DashMap;
use gtk4::ScrolledWindow;
use gtk4::prelude::*;
use smearor_model_area::AreaConfig;
use smearor_model_area::AreaType;
use smearor_swipe_launcher_plugin_api::JsonConverterRegistry;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::trace;
use tracing::warn;

/// Manages dynamic area operations at runtime.
///
/// Generic over the area backend `B`, which determines the concrete widget,
/// overlay, and container types. For GTK instances, `B = GtkBackend` uses
/// real `gtk4` types. For headless instances, `B = HeadlessBackend` uses
/// no-op types that do not require GTK initialization.
pub struct AreaManager<B: AreaBackend> {
    /// Currently managed areas keyed by ID
    areas: DashMap<String, ManagedArea<B>>,

    /// Tracks the currently visible area ID for instances where GTK
    /// widget visibility cannot be used (headless mode).
    visible_area_id: RwLock<Option<String>>,

    /// Reference to the plugin manager for loading plugins
    plugin_manager: Arc<PluginManager>,

    /// Reference to the main configuration
    pub(crate) config: Arc<SwipeLauncherConfig>,

    /// Reference to the main container
    pub(crate) main_container: Arc<RwLock<Option<B::Container>>>,
}

impl<B: AreaBackend> AreaManager<B> {
    /// Create a new AreaManager
    pub fn new(plugin_manager: Arc<PluginManager>, config: Arc<SwipeLauncherConfig>, json_converter_registry: Arc<JsonConverterRegistry>) -> Self {
        smearor_model_area::register_json_converters_in_registry(&json_converter_registry);

        Self {
            areas: DashMap::new(),
            visible_area_id: RwLock::new(None),
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

        let widget = B::create_area_widget(&self.plugin_manager, &self.config, &area_config)?;

        let overlay = B::create_overlay(&widget);

        let managed_area = ManagedArea {
            id: area_id.to_string(),
            config: area_config.clone(),
            widget: widget.clone(),
            overlay: Some(overlay.clone()),
            source_area_widget: None,
            source_area_id: None,
            is_transient: area_config.auto_close,
        };

        self.areas.insert(area_id.to_string(), managed_area);
        main_container.append_overlay(&overlay);

        if !area_config.auto_close {
            if let Ok(mut visible_id) = self.visible_area_id.write() {
                if visible_id.is_none() {
                    *visible_id = Some(area_id.to_string());
                }
            }
        }

        B::animate_addition(&overlay, &area_config.open_transition());

        trace!("Successfully added area {} with overlay", area_id);
        Ok(())
    }

    /// Remove an area with plugin cleanup
    pub fn remove_area(&self, area_id: &str) -> Result<(), RemoveAreaError> {
        trace!("Removing area {}", area_id);

        let main_container = self.get_main_container()?;

        let (area_id, managed_area) = self.areas.remove(area_id).ok_or_else(|| RemoveAreaError::AreaNotFound(area_id.to_string()))?;

        if managed_area.is_transient {
            if let Ok(mut visible_id) = self.visible_area_id.write() {
                let restore_id = managed_area
                    .source_area_id
                    .filter(|id| self.areas.contains_key(id))
                    .or_else(|| self.areas.iter().find(|entry| !entry.is_transient).map(|entry| entry.key().to_string()));
                *visible_id = restore_id;
            }
        }

        for plugin_entry in &managed_area.config.plugins {
            let namespaced_id = self.plugin_manager.namespaced_plugin_id(&plugin_entry.id);
            self.plugin_manager.unload_plugin(&namespaced_id);
        }

        let overlay_clone = managed_area.overlay.clone();
        let main_container_clone = main_container.clone();
        let area_id_clone = area_id.to_string();
        let is_transient = managed_area.is_transient;
        let source_area_widget_clone = managed_area.source_area_widget.clone();
        let source_area_overlay_clone = if is_transient {
            if let Some(ref source_widget) = managed_area.source_area_widget {
                self.find_overlay_for_widget(source_widget)
            } else {
                None
            }
        } else {
            None
        };

        let close_transition = managed_area.config.close_transition();

        B::animate_removal(
            &managed_area.widget,
            &close_transition,
            Box::new(move || {
                if is_transient {
                    if let Some(overlay) = &overlay_clone {
                        if let Some(source_overlay) = &source_area_overlay_clone {
                            source_overlay.area_remove_overlay(overlay);
                        }
                    }
                    if let Some(ref source_widget) = source_area_widget_clone {
                        source_widget.area_remove_css_class("scroll-area-transparent");
                        trace!("Restored source area widget visibility for {}", area_id_clone);
                    }
                } else {
                    if let Some(overlay) = &overlay_clone {
                        main_container_clone.remove_overlay(overlay);
                    }
                }
                trace!("Successfully removed area {}", area_id_clone);
            }),
        );

        trace!("Successfully initiated removal of area {}", area_id);
        Ok(())
    }

    /// Remove all area overlays and managed area entries without unloading plugins.
    ///
    /// Used during layout rebuilds where plugins will be reused by the new
    /// areas. Unlike `remove_all_areas_immediate`, this does NOT call
    /// `unload_plugin` — plugins remain loaded in the plugin manager.
    pub fn remove_all_areas_keep_plugins(&self) {
        debug!("Removing all areas (keeping plugins)");

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
                if let Some(overlay) = &managed_area.overlay {
                    main_container.remove_overlay(overlay);
                }
                debug!("Removed area {} (keeping plugins)", area_id);
            }
        }

        if let Ok(mut visible_id) = self.visible_area_id.write() {
            *visible_id = None;
        }
    }

    /// Remove all areas synchronously without animation.
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
                    main_container.remove_overlay(overlay);
                }
                debug!("Removed area {} immediately", area_id);
            }
        }
    }

    /// Remove all areas immediately without animation, unloading plugins.
    pub fn clear_areas(&self) {
        self.remove_all_areas_immediate();
    }

    /// Add a transient area with auto-close detection
    pub fn add_transient_area(&self, area_id: &str, area_config: AreaConfig, sender_id: Option<&str>) -> Result<(), AddAreaError> {
        trace!("Adding transient area {} from sender {:?}", area_id, sender_id);

        if self.areas.contains_key(area_id) {
            return Err(AddAreaError::AreaAlreadyExists(area_id.to_string()));
        }

        let mut config = area_config;
        config.auto_close = true;

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

        let source = if let Some(sender) = sender_id {
            self.find_area_source_containing_plugin(sender)
        } else {
            AreaSource::none()
        };

        if let Some(ref widget) = source.widget {
            widget.area_add_css_class("scroll-area-transparent");
            trace!("Made source area widget transparent for {}", area_id);
        }

        let widget = B::create_area_widget(&self.plugin_manager, &self.config, &config)?;

        let overlay = B::create_overlay(&widget);

        let managed_area = ManagedArea {
            id: area_id.to_string(),
            config: config.clone(),
            widget: widget.clone(),
            overlay: Some(overlay.clone()),
            source_area_widget: source.widget.clone(),
            source_area_id: source.area_id.clone(),
            is_transient: true,
        };

        self.areas.insert(area_id.to_string(), managed_area);

        if let Ok(mut visible_id) = self.visible_area_id.write() {
            *visible_id = Some(area_id.to_string());
        }

        if let Some(source_overlay) = source.overlay {
            source_overlay.area_add_overlay(&overlay);
            trace!("Added transient area {} overlay to source area overlay", area_id);
        } else {
            warn!("No source area overlay found for transient area {}", area_id);
        }

        B::animate_addition(&overlay, &config.open_transition());

        trace!("Successfully added transient area {}", area_id);
        Ok(())
    }

    /// Find the overlay, widget, and area ID of the area that contains a specific plugin.
    fn find_area_source_containing_plugin(&self, plugin_id: &str) -> AreaSource<B> {
        let raw_plugin_id = plugin_id.rsplit_once(':').map(|(_, id)| id).unwrap_or(plugin_id);
        for managed_area in &self.areas {
            if managed_area.config.plugins.iter().any(|p| p.id == raw_plugin_id) {
                return AreaSource {
                    overlay: managed_area.overlay.clone(),
                    widget: Some(managed_area.widget.clone()),
                    area_id: Some(managed_area.id.clone()),
                };
            }
        }
        AreaSource::none()
    }

    /// Find the overlay that has a specific widget as its child
    fn find_overlay_for_widget(&self, widget: &B::Widget) -> Option<B::Overlay> {
        for managed_area in &self.areas {
            if let Some(ref overlay) = managed_area.overlay {
                if let Some(child) = overlay.area_child() {
                    if &child == widget {
                        return Some(overlay.clone());
                    }
                }
            }
        }
        None
    }

    /// Check if an area exists
    pub fn has_area(&self, area_id: &str) -> bool {
        self.areas.contains_key(area_id)
    }

    /// Find the area that contains a button whose click_payload references
    /// the given area_id.
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
            area.widget.area_set_visible(true);
            debug!("Opened existing area {}", area_id);
            return Ok(());
        }
        let area_config = self
            .config
            .get_area_config(area_id)
            .ok_or_else(|| format!("Area {} not found in config", area_id))?
            .clone();

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
        area.widget.area_set_visible(true);
        debug!("Opened area {}", area_id);
        Ok(())
    }

    /// Hide the area widget.
    pub fn close(&self, area_id: &str) -> Result<(), String> {
        let Some(area) = self.areas.get(area_id) else {
            return Err(format!("Area {} not found", area_id));
        };
        area.widget.area_set_visible(false);
        debug!("Closed area {}", area_id);
        Ok(())
    }

    /// Move keyboard focus to the area widget.
    pub fn focus(&self, area_id: &str) -> Result<(), String> {
        let Some(area) = self.areas.get(area_id) else {
            return Err(format!("Area {} not found", area_id));
        };
        area.widget.area_grab_focus();
        debug!("Focused area {}", area_id);
        Ok(())
    }

    /// List all managed areas with their current state.
    pub fn list_areas(&self) -> Vec<AreaInfo> {
        self.areas
            .iter()
            .map(|area| AreaInfo {
                area_id: area.key().clone(),
                visible: area.widget.area_is_visible(),
                focused: area.widget.area_has_focus(),
                position: format!("{:?}", area.config.effective_align()),
                active: true,
            })
            .collect()
    }

    /// List all configured areas (including not-yet-opened ones) with their state.
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
                        visible: managed.as_ref().is_some_and(|a| a.widget.area_is_visible()),
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
        let visible = area.widget.area_is_visible();
        area.widget.area_set_visible(!visible);
        debug!("Toggled area {} to visible={}", area_id, !visible);
        Ok(())
    }

    /// Return the configuration of an area.
    pub fn get_area_config(&self, area_id: &str) -> Result<AreaConfig, String> {
        if let Some(area) = self.areas.get(area_id) {
            return Ok(area.config.clone());
        }
        self.config
            .get_area_config(area_id)
            .cloned()
            .ok_or_else(|| format!("Area {} not found", area_id))
    }

    /// Return the plugin IDs of the currently visible area.
    pub fn visible_area_plugin_ids(&self) -> Vec<String> {
        if let Ok(visible_id) = self.visible_area_id.read() {
            if let Some(ref area_id) = *visible_id {
                if let Some(entry) = self.areas.get(area_id) {
                    return entry.config.plugins.iter().map(|p| p.id.clone()).collect();
                }
            }
        }
        for entry in &self.areas {
            if entry.widget.area_is_visible() {
                return entry.config.plugins.iter().map(|p| p.id.clone()).collect();
            }
        }
        Vec::new()
    }

    /// Return the full plugin entries of the currently visible area.
    pub fn visible_area_plugin_entries(&self) -> Vec<smearor_model_plugin::PluginEntry> {
        if let Ok(visible_id) = self.visible_area_id.read() {
            if let Some(ref area_id) = *visible_id {
                if let Some(entry) = self.areas.get(area_id) {
                    return entry.config.plugins.clone();
                }
            }
        }
        for entry in &self.areas {
            if entry.widget.area_is_visible() {
                return entry.config.plugins.clone();
            }
        }
        Vec::new()
    }

    /// Set the main container for this area manager.
    pub fn set_main_container(&self, main_container: B::Container) -> Result<(), MainContainerInitializationError> {
        let Ok(mut guard) = self.main_container.write() else {
            return Err(MainContainerInitializationError);
        };
        guard.replace(main_container);
        Ok(())
    }

    /// Get the main container.
    fn get_main_container(&self) -> Result<B::Container, MainContainerNotInitialized> {
        let Ok(guard) = self.main_container.read() else {
            return Err(MainContainerNotInitialized);
        };
        guard.clone().ok_or(MainContainerNotInitialized)
    }
}

/// GTK-specific methods for `AreaManager<GtkBackend>`.
impl AreaManager<GtkBackend> {
    /// Get the scrolled window widget for a scroll-type area.
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
}

/// Headless-specific methods for `AreaManager<HeadlessBackend>`.
impl AreaManager<HeadlessBackend> {
    // All shared methods are in the generic impl block above.
}
