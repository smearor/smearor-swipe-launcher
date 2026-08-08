use gtk4::gdk::Monitor;
use gtk4::prelude::MonitorExt;
use smearor_wrot_rotation::SmearorRotation;

use super::area_size::AreaSize;
use super::monitor::resolve_monitor;

/// Calculates the area size based on rotation and the given monitor's geometry.
/// Falls back to defaults if no monitor is available.
pub fn calculate_area_size_for_monitor(rotation: SmearorRotation, default_size: i32, monitor: &Option<Monitor>) -> AreaSize {
    let Some(monitor) = monitor else {
        return AreaSize::default();
    };
    let geometry = monitor.geometry();
    let screen_width = geometry.width();
    let screen_height = geometry.height();

    let rotation = rotation.to_degrees();
    let is_horizontal = (rotation - 0.0).abs() < 0.1 || (rotation - 180.0).abs() < 0.1;
    let is_vertical = (rotation - 90.0).abs() < 0.1 || (rotation - 270.0).abs() < 0.1;

    if is_horizontal {
        AreaSize::new(screen_width, default_size)
    } else if is_vertical {
        AreaSize::new(default_size, screen_height)
    } else {
        AreaSize::default()
    }
}

pub fn calculate_area_size(rotation: SmearorRotation, default_size: i32) -> AreaSize {
    let monitor = resolve_monitor(None);
    calculate_area_size_for_monitor(rotation, default_size, &monitor)
}
