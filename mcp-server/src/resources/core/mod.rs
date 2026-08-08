//! Core resource definitions, each implementing `ResourceDefinitionCreator`.

mod area_buttons;
mod area_list;
mod area_plugins;
mod plugin_list;

pub use area_buttons::AreaButtonsResource;
pub use area_list::AreaListResource;
pub use area_plugins::AreaPluginsResource;
pub use plugin_list::PluginListResource;
