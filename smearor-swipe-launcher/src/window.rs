use crate::SwipeLauncherConfig;
use crate::display::DEFAULT_HEIGHT;
use crate::display::calculate_area_size;
use gtk4::ApplicationWindow;
use gtk4::prelude::*;

pub fn create_window(app: &gtk4::Application, config: &SwipeLauncherConfig) -> ApplicationWindow {
    let rotation = config.launcher.rotation.rotation.unwrap_or_default();
    let area_size = calculate_area_size(rotation, DEFAULT_HEIGHT);
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Smearor Swipe Launcher")
        .default_width(area_size.width)
        .default_height(area_size.height)
        .build();
    window.add_css_class("transparent-background");
    window
}
