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
    let mut area_size = coordinated_size.unwrap_or_else(|| calculate_area_size_for_monitor(rotation, height, &monitor));

    let max_width = config.layer.max_width;
    let width_capped = max_width.is_some();
    if let Some(max_w) = max_width {
        area_size.width = area_size.width.min(max_w);
    }

    let decorated = config.show_decorations.unwrap_or(true);

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(area_size.width)
        .default_height(area_size.height)
        .decorated(decorated)
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

        if width_capped {
            set_anchors_for_rotation_capped(&window, rotation);
        } else {
            set_anchors_for_rotation(&window, rotation);
        }
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

/// Updates layer-shell anchors for a width-capped window.
/// Only the edge corresponding to the rotation is anchored (bottom for 0°,
/// left for 90°, top for 180°, right for 270°). The perpendicular edges are
/// not anchored, allowing the compositor to center the window.
pub fn set_anchors_for_rotation_capped(window: &ApplicationWindow, rotation: SmearorRotation) {
    let degrees = rotation.to_degrees();
    window.set_anchor(Edge::Bottom, (degrees - 0.0).abs() < 0.1);
    window.set_anchor(Edge::Left, (degrees - 90.0).abs() < 0.1);
    window.set_anchor(Edge::Top, (degrees - 180.0).abs() < 0.1);
    window.set_anchor(Edge::Right, (degrees - 270.0).abs() < 0.1);
    // Do not anchor the perpendicular edges — compositor centers the window.
    if (degrees - 0.0).abs() < 0.1 || (degrees - 180.0).abs() < 0.1 {
        // Horizontal rotation: don't anchor left/right
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Right, false);
    } else {
        // Vertical rotation: don't anchor top/bottom
        window.set_anchor(Edge::Top, false);
        window.set_anchor(Edge::Bottom, false);
    }
}
