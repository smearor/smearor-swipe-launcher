use crate::SwipeLauncherConfig;
use crate::area::area_info::AllAreaInfo;
use crate::area::area_info::AreaInfo;
use crate::area::area_manager::AreaManager;
use crate::area::backend::HeadlessBackend;
use crate::area::container::GtkBackend;
use crate::area::container::HeadlessContainer;
use crate::area::error::AddAreaError;
use crate::area::error::RemoveAreaError;
use smearor_model_area::AreaConfig;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageRouter;
use std::sync::Arc;

/// Enum holding either a GTK or headless `AreaManager`.
///
/// `LauncherInstance` stores an `Arc<Mutex<InstanceAreaManager>>` so that
/// callers can lock the mutex and dispatch to the appropriate backend
/// without knowing which concrete type is inside.
pub enum InstanceAreaManager {
    /// GTK backend with real `gtk4` widgets.
    Gtk(AreaManager<GtkBackend>),
    /// Headless backend with no-op widgets (no GTK required).
    Headless(AreaManager<HeadlessBackend>),
}

impl InstanceAreaManager {
    /// Create a new GTK-backed area manager.
    pub fn new_gtk(
        plugin_manager: Arc<crate::plugin::PluginManager>,
        config: Arc<SwipeLauncherConfig>,
        json_converter_registry: Arc<smearor_swipe_launcher_plugin_api::JsonConverterRegistry>,
    ) -> Self {
        Self::Gtk(AreaManager::new(plugin_manager, config, json_converter_registry))
    }

    /// Create a new headless-backed area manager.
    pub fn new_headless(
        plugin_manager: Arc<crate::plugin::PluginManager>,
        config: Arc<SwipeLauncherConfig>,
        json_converter_registry: Arc<smearor_swipe_launcher_plugin_api::JsonConverterRegistry>,
    ) -> Self {
        Self::Headless(AreaManager::new(plugin_manager, config, json_converter_registry))
    }

    /// Set the main container for a GTK area manager.
    ///
    /// Returns an error if called on a headless manager.
    pub fn set_main_container_gtk(&self, container: gtk4::Box) -> Result<(), crate::area::error::MainContainerInitializationError> {
        match self {
            Self::Gtk(manager) => manager.set_main_container(container),
            Self::Headless(_) => Err(crate::area::error::MainContainerInitializationError),
        }
    }

    /// Set the main container for a headless area manager.
    ///
    /// Returns an error if called on a GTK manager.
    pub fn set_main_container_headless(&self, container: HeadlessContainer) -> Result<(), crate::area::error::MainContainerInitializationError> {
        match self {
            Self::Gtk(_) => Err(crate::area::error::MainContainerInitializationError),
            Self::Headless(manager) => manager.set_main_container(container),
        }
    }

    /// Return a reference to the launcher config.
    pub fn config(&self) -> &Arc<SwipeLauncherConfig> {
        match self {
            Self::Gtk(manager) => &manager.config,
            Self::Headless(manager) => &manager.config,
        }
    }

    /// Check if an area exists.
    pub fn has_area(&self, area_id: &str) -> bool {
        match self {
            Self::Gtk(manager) => manager.has_area(area_id),
            Self::Headless(manager) => manager.has_area(area_id),
        }
    }

    /// Add an area from configuration.
    pub fn add_area_from_config(&self, area_id: &str, area_config: AreaConfig) -> Result<(), AddAreaError> {
        match self {
            Self::Gtk(manager) => manager.add_area_from_config(area_id, area_config),
            Self::Headless(manager) => manager.add_area_from_config(area_id, area_config),
        }
    }

    /// Add a transient area with auto-close detection.
    pub fn add_transient_area(&self, area_id: &str, area_config: AreaConfig, sender_id: Option<&str>) -> Result<(), AddAreaError> {
        match self {
            Self::Gtk(manager) => manager.add_transient_area(area_id, area_config, sender_id),
            Self::Headless(manager) => manager.add_transient_area(area_id, area_config, sender_id),
        }
    }

    /// Remove an area with plugin cleanup.
    pub fn remove_area(&self, area_id: &str) -> Result<(), RemoveAreaError> {
        match self {
            Self::Gtk(manager) => manager.remove_area(area_id),
            Self::Headless(manager) => manager.remove_area(area_id),
        }
    }

    /// Remove all area overlays without unloading plugins (for layout rebuilds).
    pub fn remove_all_areas_keep_plugins(&self) {
        match self {
            Self::Gtk(manager) => manager.remove_all_areas_keep_plugins(),
            Self::Headless(manager) => manager.remove_all_areas_keep_plugins(),
        }
    }

    /// Clear the main container reference.
    /// Prevents rebuild_areas from adding areas after stop_instance but before build_window/build_headless.
    pub fn clear_main_container(&self) {
        match self {
            Self::Gtk(manager) => manager.clear_main_container(),
            Self::Headless(manager) => manager.clear_main_container(),
        }
    }

    /// Remove all areas immediately without animation.
    pub fn remove_all_areas_immediate(&self) {
        match self {
            Self::Gtk(manager) => manager.remove_all_areas_immediate(),
            Self::Headless(manager) => manager.remove_all_areas_immediate(),
        }
    }

    /// Clear all areas (alias for `remove_all_areas_immediate`).
    pub fn clear_areas(&self) {
        match self {
            Self::Gtk(manager) => manager.clear_areas(),
            Self::Headless(manager) => manager.clear_areas(),
        }
    }

    /// Ensure an area is visible, loading it from config if needed.
    pub fn ensure_area(&self, area_id: &str) -> Result<(), String> {
        match self {
            Self::Gtk(manager) => manager.ensure_area(area_id),
            Self::Headless(manager) => manager.ensure_area(area_id),
        }
    }

    /// Show an area.
    pub fn open(&self, area_id: &str) -> Result<(), String> {
        match self {
            Self::Gtk(manager) => manager.open(area_id),
            Self::Headless(manager) => manager.open(area_id),
        }
    }

    /// Hide an area.
    pub fn close(&self, area_id: &str) -> Result<(), String> {
        match self {
            Self::Gtk(manager) => manager.close(area_id),
            Self::Headless(manager) => manager.close(area_id),
        }
    }

    /// Focus an area.
    pub fn focus(&self, area_id: &str) -> Result<(), String> {
        match self {
            Self::Gtk(manager) => manager.focus(area_id),
            Self::Headless(manager) => manager.focus(area_id),
        }
    }

    /// Toggle area visibility.
    pub fn toggle(&self, area_id: &str) -> Result<(), String> {
        match self {
            Self::Gtk(manager) => manager.toggle(area_id),
            Self::Headless(manager) => manager.toggle(area_id),
        }
    }

    /// List all managed areas.
    pub fn list_areas(&self) -> Vec<AreaInfo> {
        match self {
            Self::Gtk(manager) => manager.list_areas(),
            Self::Headless(manager) => manager.list_areas(),
        }
    }

    /// List all configured areas.
    pub fn list_all_areas(&self) -> Vec<AllAreaInfo> {
        match self {
            Self::Gtk(manager) => manager.list_all_areas(),
            Self::Headless(manager) => manager.list_all_areas(),
        }
    }

    /// Get the configuration for an area.
    pub fn get_area_config(&self, area_id: &str) -> Result<AreaConfig, String> {
        match self {
            Self::Gtk(manager) => manager.get_area_config(area_id),
            Self::Headless(manager) => manager.get_area_config(area_id),
        }
    }

    /// Return plugin IDs of the currently visible area.
    pub fn visible_area_plugin_ids(&self) -> Vec<String> {
        match self {
            Self::Gtk(manager) => manager.visible_area_plugin_ids(),
            Self::Headless(manager) => manager.visible_area_plugin_ids(),
        }
    }

    /// Return full plugin entries of the currently visible area.
    pub fn visible_area_plugin_entries(&self) -> Vec<smearor_model_plugin::PluginEntry> {
        match self {
            Self::Gtk(manager) => manager.visible_area_plugin_entries(),
            Self::Headless(manager) => manager.visible_area_plugin_entries(),
        }
    }

    /// Find the area containing a button that references the given area ID.
    pub fn find_area_containing_area_button(&self, target_area_id: &str) -> Option<String> {
        match self {
            Self::Gtk(manager) => manager.find_area_containing_area_button(target_area_id),
            Self::Headless(manager) => manager.find_area_containing_area_button(target_area_id),
        }
    }

    /// Find a sender ID for transient area opening.
    pub fn find_sender_id_for_transient(&self, source_area_id: Option<&str>) -> Option<String> {
        match self {
            Self::Gtk(manager) => manager.find_sender_id_for_transient(source_area_id),
            Self::Headless(manager) => manager.find_sender_id_for_transient(source_area_id),
        }
    }
}

impl MessageRouter for InstanceAreaManager {
    fn route(&self, envelope: &FfiEnvelope) {
        match self {
            Self::Gtk(manager) => manager.route(envelope),
            Self::Headless(manager) => manager.route(envelope),
        }
    }
}
