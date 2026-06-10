use gtk4::gdk::Display;
use gtk4::gdk::Monitor;
use gtk4::prelude::Cast;
use gtk4::prelude::DisplayExt;
use gtk4::prelude::ListModelExt;
use gtk4::prelude::MonitorExt;
use smearor_wrot_rotation::SmearorRotation;

pub const DEFAULT_WIDTH: i32 = 1024;
pub const DEFAULT_HEIGHT: i32 = 200;

pub struct AreaSize {
    pub width: i32,
    pub height: i32,
}

impl Default for AreaSize {
    fn default() -> Self {
        AreaSize {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        }
    }
}

pub fn calculate_area_size(rotation: SmearorRotation, height: i32) -> AreaSize {
    let Some(display) = Display::default() else {
        return Default::default();
    };
    let monitors = display.monitors();
    let Some(monitor) = monitors.item(0).and_then(|m| m.downcast::<Monitor>().ok()) else {
        return AreaSize::default();
    };
    let geometry = monitor.geometry();
    let screen_width = geometry.width();
    let screen_height = geometry.height();

    let rotation = rotation.to_degrees();
    let is_horizontal = (rotation - 0.0).abs() < 0.1 || (rotation - 180.0).abs() < 0.1;
    let is_vertical = (rotation - 90.0).abs() < 0.1 || (rotation - 270.0).abs() < 0.1;

    let mut width = DEFAULT_WIDTH;
    let mut height = height;

    if is_horizontal {
        width = screen_width;
        height = height;
    } else if is_vertical {
        width = height;
        height = screen_height;
    }

    AreaSize { width, height }
}
