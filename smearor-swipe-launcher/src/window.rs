use crate::config::launcher::SwipeLauncherSettings;
use crate::display::AreaSize;
use crate::display::DEFAULT_HEIGHT;
use crate::display::calculate_area_size_for_monitor;
use crate::display::resolve_monitor;
use gtk4::ApplicationWindow;
use gtk4::prelude::*;
use gtk4_layer_shell::Edge;
use gtk4_layer_shell::LayerShell;
use smearor_wrot_rotation::SmearorRotation;

/// Creates the application window with layer-shell integration.
/// If `coordinated_size` is provided, it overrides the calculated size.
pub fn create_window(app: &gtk4::Application, config: &SwipeLauncherSettings, coordinated_size: Option<AreaSize>) -> ApplicationWindow {
    let rotation = config.rotation.rotation();
    let monitor_index = config.layer.monitor;
    let monitor = resolve_monitor(monitor_index);
    let height = config.layer.exclusive_zone().unwrap_or(DEFAULT_HEIGHT);
    let area_size = coordinated_size.unwrap_or_else(|| calculate_area_size_for_monitor(rotation, height, &monitor));

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(area_size.width)
        .default_height(area_size.height)
        .build();

    window.init_layer_shell();

    window.set_monitor(monitor.as_ref());

    if let Some(layer) = &config.layer.layer {
        window.set_layer(layer.clone().into());
        if let Some(namespace) = &config.layer.namespace {
            window.set_namespace(Some(namespace));
            // namespace.as_deref().unwrap_or("smearor-swipe-launcher");
        }
        match config.layer.exclusive_zone {
            Some(0) => {
                window.set_exclusive_zone(0);
            }
            Some(pixels) => {
                window.set_exclusive_zone(pixels);
            }
            None => {
                window.auto_exclusive_zone_enable();
            }
        }

        set_anchors_for_rotation(&window, rotation);
    }

    window.add_css_class("transparent-background");
    window
}

// /// Maps the internal layer enum to the gtk4-layer-shell layer enum.
// fn map_smearor_layer(layer: SmearorLayer) -> Layer {
//     layer.into()
//     // match layer {
//     //     SmearorLayer::Background => Layer::Background,
//     //     SmearorLayer::Bottom => Layer::Bottom,
//     //     SmearorLayer::Top => Layer::Top,
//     //     SmearorLayer::Overlay => Layer::Overlay,
//     // }
// }

/// Updates layer-shell anchors based on the current screen rotation.
pub fn set_anchors_for_rotation(window: &ApplicationWindow, rotation: SmearorRotation) {
    let degrees = rotation.to_degrees();
    window.set_anchor(Edge::Bottom, (degrees - 0.0).abs() < 0.1);
    window.set_anchor(Edge::Left, (degrees - 90.0).abs() < 0.1);
    window.set_anchor(Edge::Top, (degrees - 180.0).abs() < 0.1);
    window.set_anchor(Edge::Right, (degrees - 270.0).abs() < 0.1);
}
