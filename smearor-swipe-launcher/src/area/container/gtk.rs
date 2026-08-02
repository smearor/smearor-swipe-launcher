use crate::SwipeLauncherConfig;
use crate::area::backend::area::AreaBackend;
use crate::area::container::area::AreaContainer;
use crate::area::error::CreateAreaError;
use crate::plugin_manager::PluginManager;
use gtk4::Box as GtkBox;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::glib::translate::FromGlibPtrFull;
use gtk4::prelude::*;
use smearor_model_area::AreaAlign;
use smearor_model_area::AreaConfig;
use smearor_model_area::AreaTransition;
use smearor_model_area::AreaType;
use tracing::error;
use tracing::trace;

impl AreaContainer for GtkBox {
    type Overlay = Overlay;

    fn append_overlay(&self, overlay: &Overlay) {
        self.append(overlay);
    }

    fn remove_overlay(&self, overlay: &Overlay) {
        self.remove(overlay);
    }
}

/// GTK backend using real `gtk4` widget types.
#[derive(Clone, Default)]
pub struct GtkBackend;

impl AreaBackend for GtkBackend {
    type Widget = Widget;
    type Overlay = Overlay;
    type Container = GtkBox;

    fn create_area_widget(plugin_manager: &PluginManager, config: &SwipeLauncherConfig, area_config: &AreaConfig) -> Result<Widget, CreateAreaError> {
        use gtk4::Align;
        use gtk4::Orientation;
        use gtk4::PolicyType;
        use gtk4::ScrolledWindow;

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
                add_plugins_gtk(plugin_manager, config, area_config, &inner);

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
                        add_plugins_gtk(plugin_manager, config, area_config, &plugin_container);
                    }
                    AreaAlign::Center => {
                        let left_spacer = GtkBox::builder().hexpand(true).build();
                        plugin_container.append(&left_spacer);
                        add_plugins_gtk(plugin_manager, config, area_config, &plugin_container);
                        let right_spacer = GtkBox::builder().hexpand(true).build();
                        plugin_container.append(&right_spacer);
                    }
                    AreaAlign::Right => {
                        let left_spacer = GtkBox::builder().hexpand(true).build();
                        plugin_container.append(&left_spacer);
                        add_plugins_gtk(plugin_manager, config, area_config, &plugin_container);
                    }
                }

                scrolled_window.set_child(Some(&plugin_container));
                Ok(scrolled_window.upcast())
            }
        }
    }

    fn create_overlay(child: &Widget) -> Overlay {
        let overlay = Overlay::builder().build();
        overlay.set_child(Some(child));
        overlay.add_css_class("area-overlay");
        overlay
    }

    fn animate_addition(overlay: &Overlay, transition: &AreaTransition) {
        let lt = crate::area::layout_transition::LayoutTransition::new();
        lt.animate_widget_addition(&overlay.clone().upcast::<Widget>(), transition);
    }

    fn animate_removal(widget: &Widget, transition: &AreaTransition, callback: Box<dyn Fn() + 'static>) {
        let lt = crate::area::layout_transition::LayoutTransition::new();
        lt.animate_widget_removal(widget, transition, callback);
    }
}

/// Helper to add plugin widgets to a GTK container (GTK-only).
fn add_plugins_gtk(plugin_manager: &PluginManager, _config: &SwipeLauncherConfig, area_config: &AreaConfig, container: &GtkBox) {
    for plugin_entry in &area_config.plugins {
        if plugin_entry.disabled {
            continue;
        }
        let namespaced_id = plugin_manager.namespaced_plugin_id(&plugin_entry.id);
        let Some(plugin) = plugin_manager.plugins.get(&namespaced_id) else {
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
